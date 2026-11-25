#!/usr/bin/env bash
# AREA-GYM-01: the real Bevy evidence of the gym-dressed boss arena (room 5).
#
# The PT-GATE-A room pilot `juicer` plays room 5 (`--scene f01_late --f01-room juicer`) and
# scripts/room_capture.sh renders the real window under xvfb at the ticks below, at 1x
# (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`). Every capture runs TWICE
# (two full passes) and the PNGs of the two passes must be byte-identical; each capture also
# checks that the window's final COMBAT_STATE hash equals the headless run's (the dressing and
# the exit presenter are presentation only). Output: PNGs, capture_lines.txt (one GYM_STILL line
# per still and size) and SHA256SUMS (with the offline sheets of scripts/make_gym01_sheets.py).
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-gym-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-gym-01}"
caps="${CAPTURES:-artifacts/gym01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_gym_wall.png ]] || { echo "the floor01_gym_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:tick of the pilot `juicer` (seed run): the intact room before the engage, the left crack
# held after the tick-139 left-wall crash (the stun is over, the dust gone), a phase-2 exchange,
# and the defeat (hp 0 on tick 3175) with the right wall released and the exit door open.
stills=(intact:60 left_crack:300 phase2:1000 exit_open:3230)
sizes=(640x360:1 1920x1080:3)
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    for still in "${stills[@]}"; do
      IFS=: read -r name tick <<<"$still"
      dir="$caps/pass$pass-$res-$name"
      for attempt in 1 2 3; do
        bash scripts/room_capture.sh juicer juicer "$tick:1:1" "$dir" --scale "$scale" > "$dir.txt" 2> "$dir.err" && break
        [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
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
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    IFS=: read -r name tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no
    cmp -s "$a" "$b" && same=yes
    [[ "$same" == yes ]] || fail=1
    window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    match=no
    [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$match" == yes ]] || fail=1
    cp "$a" "$out/scene_${name}_$res.png"
    echo "GYM_STILL name=$name plan=juicer tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
python3 scripts/make_gym01_sheets.py "$out" "$out/scene_intact_640x360.png" "$out/scene_left_crack_640x360.png"
(cd "$out" && sha256sum ./*.png ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
