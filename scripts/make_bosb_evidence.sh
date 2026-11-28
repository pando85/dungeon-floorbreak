#!/usr/bin/env bash
# BOSB: the real Bevy evidence of the Ball of Swine's Tusklings (`--scene f01_late --f01-room bos`).
#
# Each still is a window capture (`--capture-sequence TICK:1:1`) rendered under xvfb at 640x360 (`--scale 1`). Every still is
# captured TWICE (two full passes) and the two PNGs must be byte-identical; each capture also checks that the window's final
# COMBAT_STATE hash equals the headless run's of the same tick (the art is presentation only).
#
# Output: scene_<name>_640x360.png, capture_lines.txt (one BOSB_STILL line per still) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/bos-b)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/bos-b}"
caps="${CAPTURES:-artifacts/bos-b-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/swine-01.png && -f generated/art/floor01_bos_props.png && -f generated/art/tusk_01.png && -f generated/art/tusk_01_gown.png ]] || { echo "the swine-01 / floor01_bos_props / tusk_01 atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# pilot:name:tick
#   crash   the first crash at the left arch: the ball in its stun and the two Tusklings (tux and gown) just detached
#   tell    a Tuskling's tell hold (frame 8: head low, tusks level), the other still walking
#   kick    the first kick: the kicked Tuskling flies into the ball (`bos_kick` pilot), the contact staggers it
#   cower   the ball down and a Tuskling cowering (`bos_env` pilot: the weight and the jab, one Tuskling still alive)
stills=(
  bos_kick:crash:136
  bos_kick:tell:238
  bos_kick:kick:291
  bos_env:cower:775
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
  grep -q '^BALL_ART drawn' "$dir.log" && grep -q '^TUSK_ART drawn' "$dir.log" || { echo "the Ball's or the Tusklings' presenter did not draw: see $dir.log" >&2; exit 1; }
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
  echo "BOSB_STILL name=$name pilot=$pilot tick=$tick size=$res hero_x=$hero window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
