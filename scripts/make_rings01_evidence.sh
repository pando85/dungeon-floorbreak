#!/usr/bin/env bash
# AREA-RINGS-01: the real Bevy evidence of the Broken Ring rooms 4a ring_court and 4b ring_split.
#
# `--scene f01_late --f01-run --f01-room ROOM --ring-pilot` plays the room's library pilot
# (`f01_ring_fixtures::pilot`: the step, the walkway, the fight, the exit; the Ring rooms have no
# `--room-pilot`, and the walk-right script cannot pass the closed shutter) and renders the real window under
# xvfb at the ticks below, at 640x360 (`--scale 1`), 1920x1080 (`--scale 3`) and the 640x400 deck view
# (`--deck --scale 1`). Every capture runs TWICE (two full passes) and the PNGs of the two passes must be
# byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals the headless run's
# (the dressing and the shutter presenter are presentation only). Output: PNGs, capture_lines.txt (one
# RINGS_STILL line per still and size), the offline sheets of scripts/make_rings01_sheets.py, the measurements of
# scripts/measure_rings01.py (if RINGS_LAYOUT_DIR holds the layouts) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-rings-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   RINGS_LAYOUT_DIR=<dir written by `ring_kit::tests::dump_ring_layouts`> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-rings-01}"
caps="${CAPTURES:-artifacts/rings01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_ring_wall.png ]] || { echo "the floor01_ring_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
# One window at a time (the room_capture.sh lock).
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
xvfb() { xvfb-run -a -s '-screen 0 1920x1080x24' "$@" || true; }
# name:room:tick. The scripted pilot's hero (x, sim camera x) per tick: 4a x 235 at tick 40 (cam 0), 975 at 280 (639),
# on the walkway at 340 (x 1146), standing at x 1365 from tick 500 (cam 1041, settled), 1552 at 660, 1629 at 700 (the
# goblin at 1824 is fought here), 1752 at 740 (cam 1408, clamped), at the exit from 850; 4b x 420 at tick 100 (cam 84),
# 736 at 280 (cam 400), 921 at 340, x 1062 from 400 (cam 512, clamped).
stills=(entry_keystone:ring_court:40 great_arc_shutter:ring_court:280 walkway_under_arc:ring_court:340
        after_walkway:ring_court:530 ring_goblin:ring_court:700 full_ring:ring_court:740 exit:ring_court:850
        arc_route:ring_split:100 camp:ring_split:280 r3_breach:ring_split:340 marker_exit:ring_split:420)
# res:scale:deck-flag
sizes=(640x360:1: 1920x1080:3: 640x400:1:--deck)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    IFS=: read -r res scale deck <<<"$size"
    for still in "${stills[@]}"; do
      IFS=: read -r name room tick <<<"$still"
      dir="$caps/pass$pass-$res-$name"
      for attempt in 1 2 3; do
        rm -rf "$dir"; mkdir -p "$dir"
        xvfb "$bin" --no-audio --no-record --scale "$scale" --scene f01_late --f01-run --f01-room "$room" --ring-pilot \
          ${deck:+$deck} --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1
        grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
        [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir.log" >&2; exit 1; }
      done
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  IFS=: read -r res scale deck <<<"$size"
  for still in "${stills[@]}"; do
    IFS=: read -r name room tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no
    cmp -s "$a" "$b" && same=yes
    [[ "$same" == yes ]] || fail=1
    window="$(state "$caps/pass1-$res-$name.log")"
    "$bin" --headless-floor01-smoke --scene f01_late --f01-run --f01-room "$room" --ring-pilot ${deck:+$deck} \
      --capture-tick "$tick" > "$caps/headless-$res-$name.log" 2>&1
    headless="$(state "$caps/headless-$res-$name.log")"
    cam="$(grep '^FLOOR1_SCENE_STATE ' "$caps/headless-$res-$name.log" | tail -n 1 | sed -E 's/.* cam=([0-9]+),.*/\1/')"
    match=no
    [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$match" == yes ]] || fail=1
    cp "$a" "$out/scene_${name}_$res.png"
    echo "RINGS_STILL name=$name room=$room tick=$tick size=$res cam=$cam window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
python3 scripts/author_rings01.py gate "$out/gate" > "$out/gate/measurements.stdout" 2>&1 || true
rm -f "$out/gate/measurements.stdout"
python3 scripts/make_rings01_sheets.py "$out"
if [[ -n "${RINGS_LAYOUT_DIR:-}" ]]; then
  python3 scripts/measure_rings01.py "$RINGS_LAYOUT_DIR" "$out/measurements.txt" > /dev/null
fi
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
