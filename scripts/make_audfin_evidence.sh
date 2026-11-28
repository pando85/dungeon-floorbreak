#!/usr/bin/env bash
# AUD-FIN-01: the real Bevy evidence of the Ball's notices and the finish-aware defeat notice.
#
# Each still is a window capture (`--capture-sequence TICK:1:1`) rendered under xvfb at 640x360 (`--scale 1`). Every still is
# captured TWICE (two full passes) and the two PNGs must be byte-identical; each capture also checks that the window's final
# COMBAT_STATE hash equals the headless run's of the same tick (the notices are presentation only).
#
# Stills: the Ball's intro notice (ROLL CALL!), its wall crash (WALL WINS!), the rack pin (DEAD WEIGHT!), its defeat
# (BACON BOWLED!) and a point blank blast of a goblin (BLOWN AWAY!, the `qa_blast_breakup` fixture). The floor-end panel with the
# viewers line is a text artifact (`floor_end_panel.txt`, from the `f01_run` unit test): a scripted run cannot cross a room
# exit (ROOM-PILOT-RUN), so there is no window still of it.
#
# Output: scene_<name>_640x360.png, capture_lines.txt, floor_end_panel.txt and SHA256SUMS.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/aud-fin-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/aud-fin-01}"
caps="${CAPTURES:-artifacts/aud-fin-01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/swine-01.png ]] || { echo "the swine-01 atlas is missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:tick:args...   (the args select the scene)
bos=(--scene f01_late --f01-room bos --room-pilot bos_weight --companion off)
blast=(--fixture qa_blast_breakup)
stills=(
  intro:50:bos
  crash:470:bos
  pinned:240:bos
  defeat:1685:bos
  blown_away:70:blast
)
res=640x360
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | grep -oE ' hash=[0-9a-f]+' | head -n 1 | cut -d= -f2; }
scene_args() { if [[ "$1" == bos ]]; then echo "${bos[@]}"; else echo "${blast[@]}"; fi; }
capture() {
  local scene="$1" tick="$2" dir="$3"
  local -a sargs
  read -r -a sargs <<<"$(scene_args "$scene")"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale 1 "${sargs[@]}" \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $scene $tick failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke "${sargs[@]}" --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r name tick scene <<<"$still"
    frame="$(capture "$scene" "$tick" "$caps/pass$pass-$name")"
    cp "$frame" "$caps/pass$pass-${name}.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r name tick scene <<<"$still"
  a="$caps/pass1-${name}.png" b="$caps/pass2-${name}.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  echo "AUDFIN_STILL name=$name scene=$scene tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
if [[ "${PANEL:-on}" == on ]]; then
  cargo test --locked --lib the_floor_end_panel_shows_the_viewers_at_the_exit -- --nocapture 2>/dev/null \
    | grep '^AUDFIN_PANEL' > "$out/floor_end_panel.txt"
fi
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
