#!/usr/bin/env bash
# GUILD-VERB-01: the real Bevy evidence of the Interact verb in `act1_guild`: Carl's hand verb on the
# grant strongbox mid-animation, the strongbox open, and Mordecai's talk key held by an Interact press
# (Carl stands 57 px from his column, outside the 48 px proximity key, so the key can only come from
# the press).
#
# Every still is rendered in a real window under xvfb at 640x360 (`--scale 1`, `--act1-pilot interact`)
# and captured TWICE (two full passes); the two PNGs must be byte-identical, and each capture checks that
# the window's final COMBAT_STATE hash equals the headless run's of the same tick (the art is
# presentation only). Output: PNGs, capture_lines.txt (one GUILDV01_STILL line per still) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   OUT=<evidence dir, default docs/evidence/guild-verb-01>
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/guild-verb-01}"
caps="${CAPTURES:-artifacts/guildverb01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/carl_book1_h52_interact.png && -f generated/art/donut_h40_interact.png ]] \
  || { echo "the atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
res=640x360
# name:room:plan:tick  (the pilot walks Carl to x 316, presses Interact on the strongbox (clock 0 at tick
# 67), and after the verb presses it again for the guide)
#   interact_carl_reach  tick 71: clock 4, the reach (u0) before the grip, the strongbox still closed
#   interact_carl_haul   tick 81: clock 14, the haul-down key (u2) of the hand verb, the strongbox open
#   interact_box_open    tick 97: the last verb tick (let go), the box open since the contact tick (72)
#   interact_guide_talks tick 130: Carl 57 px from the guide, his talk key held by the second press
stills=(
  interact_carl_reach:act1_guild:interact:71
  interact_carl_haul:act1_guild:interact:81
  interact_box_open:act1_guild:interact:97
  interact_guide_talks:act1_guild:interact:130
)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
capture() { # room plan tick dir -> prints the frame path
  local room="$1" plan="$2" tick="$3" dir="$4" attempt
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale 1 \
      --scene f01_late --f01-room "$room" --act1-pilot "$plan" --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $room $tick failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" --act1-pilot "$plan" --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  find "$dir" -name 'frame_*.png' | sort | tail -n 1
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r name room plan tick <<<"$still"
    frame="$(capture "$room" "$plan" "$tick" "$caps/pass$pass-$name")"
    [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r name room plan tick <<<"$still"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  echo "GUILDV01_STILL name=$name room=$room plan=$plan tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
