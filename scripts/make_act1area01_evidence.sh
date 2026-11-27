#!/usr/bin/env bash
# AREA-ACT1-01: the real Bevy evidence of the dressed act-1 rooms (`act1_chase`, `act1_shore`, `act1_guild`).
#
# Each still is the ACT1 capture command (`--scene f01_late --f01-room "$room" --act1-pilot "$pilot" --companion off
# --capture-sequence TICK:1:1`) rendered in a real window under xvfb at 640x360 (`--scale 1`). Every still is captured
# TWICE (two full passes) and the two PNGs must be byte-identical; each capture also checks that the window's final
# COMBAT_STATE hash equals the headless run's of the same tick (the art is presentation only) and, where the ACT1-A or
# ACT1-B evidence (docs/evidence/act1/capture_lines.txt) captured the same room, pilot, tick and size, that it equals their
# hash too. The camera eases per frame, so a still is defined by its room, pilot and tick; the drawn view (left, top) is
# measured from landmarks afterwards (scripts/make_act1area01_sheets.py), never taken from the logged `cam`.
#
# Output: the PNGs `scene_<name>_640x360.png`, capture_lines.txt (one ACT1_ART_STILL line per still), the offline sheets and
# measurements of scripts/make_act1area01_sheets.py (when ACT1_LAYOUT_DIR holds the layouts written by the ignored test
# `act1_kit::tests::dump_act1_layouts`), and SHA256SUMS. `gate/` (the design step's composites) is left as it is.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-act1-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   ACT1_LAYOUT_DIR=<dir written by the ignored test act1_kit::tests::dump_act1_layouts> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-act1-01}"
caps="${CAPTURES:-artifacts/act1-art-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_act1_keys.png && -f generated/art/floor01_act1_soffit.png ]] || { echo "the floor01_act1_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:pilot:name:tick. The ticks that ACT1-A or ACT1-B also captured keep their hash comparison (act1_match).
#   chase_spawn            S1 at spawn: the soffit and the first timber set intact, the bricked bay behind the breach
#   chase_s2_breach_heroes S2: Carl and Donut in front of the breach before the machine comes (the void behind them)
#   chase_s2_emergence     S2: the machine in front of the gallery breach (the rig drawn over the void)
#   chase_s3_timbers       S3: the soffit over the lane and a timber set intact (the torches inside the sets are gone)
#   chase_s3_after_first_smash  tick 510, the first timber set (x 1104..1183) already smashed by the front and out of frame to the
#                          left (the camera is at x >= 1216 under every evidence pilot), the second set intact ahead
#   chase_s4_alcove        S4: the alcove niche on its ledge, the brightest shape of the lane
#   chase_s5_barricade_*   S5: the barricade intact, then broken by the front (the smashed frame stays shown)
#   chase_s5_after_defeat  Carl defeated, the machine breaks the barricade during the defeat delay (the break is shown)
#   chase_s5_after_respawn the second try: the respawn closed the lock, the barricade is intact again
#   chase_wedge            the machine wedged in the arch (greybox wedge draw, kept)
#   shore_spawn            the shore at spawn: the wedged machine drawn from the chase wedge's pose
#   guild_door_*           the guild with the door closed and the box closed, then open and the box open
stills=(
  act1_chase:hide:chase_spawn:20
  act1_chase:hide:chase_s2_breach_heroes:235
  act1_chase:watch:chase_s2_emergence:470
  act1_chase:hide:chase_s3_timbers:480
  act1_chase:hide:chase_s3_after_first_smash:510
  act1_chase:hide:chase_s4_alcove:600
  act1_chase:hide:chase_s5_barricade_intact:825
  act1_chase:hide:chase_s5_barricade_broken:880
  act1_chase:skip:chase_s5_after_defeat:925
  act1_chase:skip:chase_s5_after_respawn:1350
  act1_chase:hide:chase_wedge:1330
  act1_shore:strike:shore_spawn:20
  act1_guild:donut:guild_door_closed:60
  act1_guild:donut:guild_door_open:112
)
res=640x360
scale=1
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
prev_hash() { # room pilot tick -> the ACT1-A / ACT1-B capture's window hash for the same pilot, tick and size, or empty
  grep -hE "^ACT1_STILL .* room=$1 pilot=$2 tick=$3 size=$res " docs/evidence/act1/capture_lines.txt 2>/dev/null \
    | head -n 1 | sed -E 's/.* window_hash=([0-9a-f]+) .*/\1/' || true
}
# capture ROOM PILOT TICK DIR: one window frame at TICK (the last frame of the sequence TICK:1:1)
capture() {
  local room="$1" pilot="$2" tick="$3" dir="$4"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale "$scale" \
      --scene f01_late --f01-room "$room" --act1-pilot "$pilot" --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $room $tick failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" --act1-pilot "$pilot" --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r room pilot name tick <<<"$still"
    frame="$(capture "$room" "$pilot" "$tick" "$caps/pass$pass-$name")"
    cp "$frame" "$caps/pass$pass-${name}.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r room pilot name tick <<<"$still"
  a="$caps/pass1-${name}.png" b="$caps/pass2-${name}.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  prev="$(prev_hash "$room" "$pilot" "$tick")"
  prevmatch=n/a
  if [[ -n "$prev" ]]; then prevmatch=no; [[ "$prev" == "$window" ]] && prevmatch=yes; fi
  [[ "$same" == yes && "$match" == yes && "$prevmatch" != no ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  hero="$(grep '^FLOOR1_SCENE_STATE' "$caps/pass1-$name.log" | tail -n 1 | sed -E 's/.* hero=([0-9.-]+),.*/\1/')"
  cam="$(grep '^FLOOR1_SCENE_STATE' "$caps/pass1-$name.log" | tail -n 1 | sed -E 's/.* cam=([0-9-]+),.*/\1/')"
  echo "ACT1_ART_STILL name=$name room=$room pilot=$pilot tick=$tick size=$res hero_x=$hero cam_x=$cam window_hash=$window headless_hash=$headless hash_match=$match act1_hash=${prev:-none} act1_match=$prevmatch passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
python3 scripts/make_act1area01_sheets.py "$out" "${ACT1_LAYOUT_DIR:-}"
(cd "$out" && find . -path ./gate -prune -o -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
