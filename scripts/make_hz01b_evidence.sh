#!/usr/bin/env bash
# AREA-HAZARDS-01B: the in-scene evidence of the acid splash, the teal pool, the corrosion, the Rot nest and the
# spent drip in rooms 4 (hazards) and 6 (mastery), and of the ring tape backing. Spec:
# docs/02-art/area-hazards-01b-spec.md.
#
# The room pilots play the rooms and scripts/room_capture.sh renders the real Bevy window under xvfb at 1x
# (640x360, `--scale 1`) at the ticks below. Every capture is run TWICE (two full passes) and the PNGs of the two
# passes must be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals the
# headless run's (the presentation is read-only). It also renders one game-speed GIF of the room 4 splash.
# Output: the PNGs, the GIF, capture_lines.txt (one HZ01B_STILL line per still) and SHA256SUMS in $OUT/scene.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh: generated/art/f01-dfx-*.png), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-hazards-01b)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-hazards-01b}/scene"
caps="${CAPTURES:-artifacts/hz01b}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
for sheet in acid-splash corr-face corr-top corr-wall rot-nest rot-trail rot-drip; do
  [[ -f "generated/art/f01-dfx-$sheet.png" ]] || { echo "generated/art/f01-dfx-$sheet.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
done
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:plan:name:tick. What happens when (seed-free pilots):
#  - room 4, `direct`: Carl walks into the first pool and is hurt on tick 41 (x 238, left of the pool's centre: a
#    mirrored splash) and again on tick 100 (x 299, right of the centre: the splash is not mirrored); the
#    crown frame shows on age 5.
#  - room 4, `haz_spectacular`: the slime under the door is defeated on tick 164 (its pool drains on 164..223 and
#    is gone after), sticker 0 is provoked on 373 and blasts on 397 (its spent drip runs from 397), Carl's stick
#    lands on 522 and its 64 px ring burns by the lit brick and the torch until 634.
#  - room 6, `mastery`: the slime is defeated on 101 (the pool drains until about 161), Carl is hurt by the
#    draining pool on tick 123, the sticker is provoked on 677 and blasts on 701.
stills=(
  hazards:haz_spectacular:r4_slime_pool:120
  hazards:direct:r4_splash_left_entry:46
  hazards:direct:r4_splash_right_entry:105
  hazards:haz_spectacular:r4_rot_ring_march:385
  hazards:haz_spectacular:r4_dyn_ring_march_lit_brick:560
  hazards:haz_spectacular:r4_dyn_ring_hot_lit_brick:630
  hazards:haz_spectacular:r4_corrosion_drained:240
  hazards:haz_spectacular:r4_nest_drip:440
  mastery:mastery:r6_slime_pool:90
  mastery:mastery:r6_splash:128
  mastery:mastery:r6_corrosion_drained:210
  mastery:mastery:r6_ring_march_dark_brick:690
  mastery:mastery:r6_nest_drip:780
)
capture() { # room plan seq dir [flags]
  local room="$1" plan="$2" seq="$3" dir="$4"
  shift 4
  # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the
  # machine, not the run, so a capture is retried up to three times. A hash mismatch is not retried
  # away: it fails the checks below.
  for attempt in 1 2 3; do
    bash scripts/room_capture.sh "$room" "$plan" "$seq" "$dir" "$@" > "$dir.txt" 2> "$dir.err" && return 0
    [[ "$attempt" -lt 3 ]] || { echo "capture $dir failed 3 times: see $dir/run.log" >&2; exit 1; }
  done
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r room plan name tick <<<"$still"
    dir="$caps/pass$pass-$name"
    capture "$room" "$plan" "$tick:1:1" "$dir" --scale 1
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == "640x360" ]] || { echo "bad size: $frame" >&2; exit 1; }
    grep -q '^DANGER_FX sheets=8/8' "$dir/run.log" || { echo "the danger FX atlases did not load: $dir/run.log" >&2; exit 1; }
    grep -q '^HAZARD_DECALS sheets=6/6' "$dir/run.log" || { echo "the hazard decal atlases did not load: $dir/run.log" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r room plan name tick <<<"$still"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no
  cmp -s "$a" "$b" && same=yes
  [[ "$same" == yes ]] || fail=1
  window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$name.txt")"
  headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$name.txt")"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$match" == yes ]] || fail=1
  cp "$a" "$out/${name}_640x360.png"
  echo "HZ01B_STILL room=$room plan=$plan name=$name tick=$tick size=640x360 window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
# The splash at game speed (room 4, Carl's first entry, ticks 40..62 every tick).
dir="$caps/gif-splash"
capture hazards direct 40:1:23 "$dir" --scale 1
python3 scripts/room_gif.py "$dir" "$out/splash_room4_640x360.gif" --step-ticks 1
window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
match=no
[[ -n "$window" && "$window" == "$headless" ]] && match=yes
[[ "$match" == yes ]] || fail=1
echo "HZ01B_GIF room=hazards plan=direct name=splash_room4 ticks=40..62 step=1 window_hash=$window headless_hash=$headless hash_match=$match" >> "$out/capture_lines.txt"
(cd "$out" && sha256sum ./*.png ./*.gif ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
