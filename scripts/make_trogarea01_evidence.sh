#!/usr/bin/env bash
# AREA-TROG-01 stages 1 and 2: the real Bevy evidence of the troglodyte run (`trog_run`) and the Test of Strength
# (`trog_strength`).
#
# Each still is the TROG-A capture command (`--scene f01_late --f01-room "$room" --trog-pilot carl --companion off
# --capture-sequence TICK:1:1`) rendered in a real window under xvfb at 640x360 (`--scale 1`) and 1920x1080
# (`--scale 3`). Every still is captured TWICE (two full passes) and the two PNGs must be byte-identical; each capture also
# checks that the window's final COMBAT_STATE hash equals the headless run's of the same tick (the art is presentation
# only) and, where TROG-A (docs/evidence/trog/capture_lines.txt) or TROG-B (docs/evidence/trog-b/capture_lines.txt) captured the
# same room, pilot, tick and size, that it equals their hash too. The camera eases per frame, so a still is defined by its room, pilot and tick; the drawn view (left) is
# measured from landmarks afterwards (scripts/make_trogarea01_sheets.py), never taken from the logged `cam`.
#
# Output: the PNGs `scene_<name>_<size>.png`, capture_lines.txt (one TROG_ART_STILL line per still and size), the offline
# sheets and measurements of scripts/make_trogarea01_sheets.py (when TROG_LAYOUT_DIR holds the layouts written by the
# ignored test `trog_kit::tests::dump_trog_layouts`), and SHA256SUMS. `gate/` (the design step's composites) is left as it is.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-trog-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   TROG_LAYOUT_DIR=<dir written by the ignored test trog_kit::tests::dump_trog_layouts> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-trog-01}"
caps="${CAPTURES:-artifacts/trog-art-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_trog_run_wall.png && -f generated/art/floor01_trog_strength.png ]] || { echo "the floor01_trog_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:name:tick (pilot `carl`, the TROG-A / TROG-B evidence pilot). The ticks that TROG-A or TROG-B also captured keep
# their names (the strength stills are prefixed `strength_`): the hash must match theirs.
#   t0_spawn          T0: the slam crack and the knuckle marks on bricked bay 1, the refuge face, Carl and Donut at the spawn
#   *_dormant         a troglodyte asleep at its roster x (the gated stills of TROG-ART-A12)
#   strength_open     the shelf face and the Virtuoso asleep at 496 (the bench, from x 640, is just off the right edge)
#   strength_bench_*  Carl beside the bench, then landing on its top: the threshold, the gym bay with the shards, the rack
stills=(
  trog_run:t0_spawn:20
  trog_run:pygmy_dormant:80
  trog_run:pygmy_tell:125
  trog_run:pygmy_leap:144
  trog_run:pygmy_land:152
  trog_run:basher_dormant:440
  trog_run:basher_tell:520
  trog_run:basher_smash:543
  trog_run:virtuoso_dormant:840
  trog_run:virtuoso_tell:870
  trog_run:virtuoso_lash:893
  trog_run:exit_open:960
  trog_strength:strength_open:30
  trog_strength:strength_virtuoso_tell:105
  trog_strength:strength_bench_beside:230
  trog_strength:strength_bench_climb:270
  trog_strength:strength_basher_tell:360
  trog_strength:strength_basher_smash:381
  trog_strength:strength_exit_open:450
)
sizes=(640x360:1 1920x1080:3)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
prev_hash() { # room tick size -> the TROG-A / TROG-B capture's window hash for the same pilot, tick and size, or empty
  local file=docs/evidence/trog/capture_lines.txt
  [[ "$1" == trog_strength ]] && file=docs/evidence/trog-b/capture_lines.txt
  grep -hE "^TROG_STILL .* pilot=carl tick=$2 size=$3 " "$file" 2>/dev/null \
    | head -n 1 | sed -E 's/.* window_hash=([0-9a-f]+) .*/\1/' || true
}
# capture ROOM TICK RES SCALE DIR: one window frame at TICK (the last frame of the sequence TICK:1:1)
capture() {
  local room="$1" tick="$2" res="$3" scale="$4" dir="$5"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale "$scale" \
      --scene f01_late --f01-room "$room" --trog-pilot carl --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $room $tick at $res failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" --trog-pilot carl --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    IFS=: read -r res scale <<<"$size"
    for still in "${stills[@]}"; do
      IFS=: read -r room name tick <<<"$still"
      frame="$(capture "$room" "$tick" "$res" "$scale" "$caps/pass$pass-$res-$name")"
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for size in "${sizes[@]}"; do
  IFS=: read -r res scale <<<"$size"
  for still in "${stills[@]}"; do
    IFS=: read -r room name tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no; cmp -s "$a" "$b" && same=yes
    window="$(state "$caps/pass1-$res-$name.log")"
    headless="$(state "$caps/pass1-$res-$name.headless.log")"
    match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    ta="$(prev_hash "$room" "$tick" "$res")"
    tamatch=n/a
    if [[ -n "$ta" ]]; then tamatch=no; [[ "$ta" == "$window" ]] && tamatch=yes; fi
    tag=troga; [[ "$room" == trog_strength ]] && tag=trogb
    [[ "$same" == yes && "$match" == yes && "$tamatch" != no ]] || fail=1
    carl="$(grep '^FLOOR1_SCENE_STATE' "$caps/pass1-$res-$name.log" | tail -n 1 | sed -E 's/.* hero=([0-9.-]+),.*/\1/')"
    cp "$a" "$out/scene_${name}_$res.png"
    echo "TROG_ART_STILL name=$name room=$room pilot=carl tick=$tick size=$res carl_x=$carl window_hash=$window headless_hash=$headless hash_match=$match ${tag}_hash=${ta:-none} ${tag}_match=$tamatch passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
python3 scripts/make_trogarea01_sheets.py "$out" "${TROG_LAYOUT_DIR:-}"
(cd "$out" && find . -path ./gate -prune -o -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
