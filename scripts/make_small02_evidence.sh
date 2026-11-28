#!/usr/bin/env bash
# SMALL-02: the real Bevy evidence of the presentation items of the Floor 1 small follow-ups bundle:
# Mordecai's idle loop and talk key (`act1_guild`), the Boiler's landing plume beside the flush drum
# (ST-OQ1) and the Hoarder's post hit flash and splinter burst (HC-OQ3).
#
# Every still is rendered in a real window under xvfb at 640x360 (`--scale 1`) and captured TWICE (two full
# passes); the two PNGs must be byte-identical, and each capture checks that the window's final COMBAT_STATE hash
# equals the headless run's of the same tick (the art is presentation only). Output: PNGs, capture_lines.txt (one
# SMALL02_STILL line per still) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   OUT=<evidence dir, default docs/evidence/small-02>
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/small-02}"
caps="${CAPTURES:-artifacts/small02-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/mordecai_h44.png && -f generated/art/floor01_splinter_burst.png ]] \
  || { echo "the atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
res=640x360
# kind:name:room:plan:first:step:count
#   act1  = `--act1-pilot PLAN` (a single frame at FIRST); room = a room_capture.sh sequence of the last frame.
#   mordecai_idle_0   tick 20: the first idle frame (Carl far away)
#   mordecai_idle_2   tick 70: the inhale hold (idle frame 2)
#   mordecai_talk     tick 100: Carl 37 px from his column, so the talk key shows
#   boiler_plume      tick 697: the rolling head on the left, the right plume dropped at the flush drum
#   hoarder_post_hit  tick 452: two ticks after the counted hit on the J-B post (flash, burst frame 0)
stills=(
  act1:mordecai_idle_0:act1_guild:donut:20:1:1
  act1:mordecai_idle_2:act1_guild:donut:70:1:1
  act1:mordecai_talk:act1_guild:donut:100:1:1
  room:boiler_plume:boiler:chain:693:1:5
  room:hoarder_post_hit:hoarder:direct:446:1:7
)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
capture() { # kind room plan first step count dir -> prints the frame path
  local kind="$1" room="$2" plan="$3" first="$4" step="$5" count="$6" dir="$7" last attempt
  last=$((first + step * (count - 1)))
  if [[ "$kind" == act1 ]]; then
    for attempt in 1 2 3; do
      rm -rf "$dir"; mkdir -p "$dir"
      xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale 1 \
        --scene f01_late --f01-room "$room" --act1-pilot "$plan" --companion off \
        --capture-sequence "$first:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
      grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
      [[ "$attempt" -lt 3 ]] || { echo "capture $room $first failed 3 times: see $dir.log" >&2; exit 1; }
    done
    "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" --act1-pilot "$plan" --companion off \
      --capture-tick "$last" > "$dir.headless.log" 2>&1
  else
    for attempt in 1 2 3; do
      if BIN="$bin" scripts/room_capture.sh "$room" "$plan" "$first:$step:$count" "$dir" > "$dir.out" 2>&1; then
        break
      fi
      [[ "$attempt" -lt 3 ]] || { echo "capture $room $plan failed 3 times: see $dir.out" >&2; exit 1; }
    done
    cp "$dir/run.log" "$dir.log"; cp "$dir/headless.log" "$dir.headless.log"
  fi
  find "$dir" -name 'frame_*.png' | sort | tail -n 1
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r kind name room plan first step count <<<"$still"
    frame="$(capture "$kind" "$room" "$plan" "$first" "$step" "$count" "$caps/pass$pass-$name")"
    [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r kind name room plan first step count <<<"$still"
  tick=$((first + step * (count - 1)))
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  echo "SMALL02_STILL name=$name room=$room plan=$plan sequence=$first:$step:$count tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
