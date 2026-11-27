#!/usr/bin/env bash
# BOS-ART: the real Bevy evidence of the dressed Ball of Swine room (`--scene f01_late --f01-room bos`).
#
# Each still is a window capture (`--capture-sequence TICK:1:1`) rendered under xvfb at 640x360 (`--scale 1`). Every still is
# captured TWICE (two full passes) and the two PNGs must be byte-identical; each capture also checks that the window's final
# COMBAT_STATE hash equals the headless run's of the same tick (the art is presentation only).
#
# Output: scene_<name>_640x360.png, capture_lines.txt (one BOS_ART_STILL line per still), contact_x4_weight.png (the chain
# weight key at x4 and 1x, from scripts/author_bosart.py), measurements.txt (the key pins and the placement boxes) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/bos-art)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/bos-art}"
caps="${CAPTURES:-artifacts/bos-art-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/swine-01.png && -f generated/art/floor01_bos_props.png ]] || { echo "the swine-01 / floor01_bos_props atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# pilot:name:tick
#   idle      the ball before the gate closes (frame 0 idle), the weight hanging on its chain and tie
#   tell      the roll tell (frame 10, the coil)
#   roll      mid-roll (one of the six native roll frames)
#   crash     the crash into the left arch (frame 12, the flat face)
#   stun      the stun (frame 13), the weight still hung (the post not hit)
#   pinned    the weight dropped on the stunned ball (`bos_weight` pilot: three post hits)
#   exit_open the ball down (the heap) and the stairwell door open behind the right wall
stills=(
  bos:idle:20
  bos:tell:55
  bos:roll:100
  bos:crash:135
  bos:stun:150
  bos_weight:pinned:200
  bos:exit_open:1900
)
res=640x360
scale=1
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+) .*/\1/'; }
# capture PILOT TICK DIR: one window frame at TICK (the last frame of the sequence TICK:1:1)
capture() {
  local pilot="$1" tick="$2" dir="$3"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale "$scale" \
      --scene f01_late --f01-room bos --room-pilot "$pilot" --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $pilot $tick failed 3 times: see $dir.log" >&2; exit 1; }
  done
  grep -q '^BALL_ART drawn' "$dir.log" || { echo "the Ball's presenter did not draw: see $dir.log" >&2; exit 1; }
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room bos --room-pilot "$pilot" --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r pilot name tick <<<"$still"
    frame="$(capture "$pilot" "$tick" "$caps/pass$pass-$name")"
    cp "$frame" "$caps/pass$pass-${name}.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r pilot name tick <<<"$still"
  a="$caps/pass1-${name}.png" b="$caps/pass2-${name}.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  hero="$(grep '^FLOOR1_SCENE_STATE' "$caps/pass1-$name.log" | tail -n 1 | sed -E 's/.* hero=([0-9.-]+),.*/\1/')"
  echo "BOS_ART_STILL name=$name pilot=$pilot tick=$tick size=$res hero_x=$hero window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
python3 scripts/author_bosart.py gate "$out/contact_x4_weight.png"
{ python3 scripts/author_bosart.py measure; } > "$out/measurements.txt"
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
