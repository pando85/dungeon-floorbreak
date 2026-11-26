#!/usr/bin/env bash
# F01 danger FX: the in-scene evidence of the danger art (acid pool, blast reach rings, blast waves,
# room 5 gate). Spec: docs/02-art/floor01-danger-fx-spec.md.
#
# Two room pilots play the rooms and scripts/room_capture.sh renders the real Bevy window under xvfb
# at the ticks below, at 1x (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`):
#   - room 4 (hazards) with the `haz_spectacular` plan: the acid pools (full, draining), a Rot Sticker
#     warning ring (marching, hot), its 48 px blast wave, Carl's landed stick with its 64 px reach ring
#     (marching, hot) and the 64 px blast wave with the chained sticker's ring;
#   - room 5 (juicer) with the `juicer` plan: the entrance gate raised, closed, and closed with the
#     Juicer crashed against it.
# Every capture is run TWICE (two full passes) and the PNGs of the two passes must be byte-identical;
# each capture also checks that the window's final COMBAT_STATE hash equals the headless run's (the
# presentation is read-only). It also renders one game-speed GIF of the dynamite beat (1x, every 2
# ticks). Output: the PNGs, the GIF, capture_lines.txt (one DANGER_FX_STILL line per still and size)
# and SHA256SUMS in $OUT/scene.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh: generated/art/f01-dfx-*.png), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/f01-danger-fx)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-danger-fx}/scene"
caps="${CAPTURES:-artifacts/danger-fx}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
for sheet in acid ring-r48 ring-r64 wave-r48 wave-r64 gate; do
  [[ -f "generated/art/f01-dfx-$sheet.png" ]] || { echo "generated/art/f01-dfx-$sheet.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
done
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:plan:name:tick. Ticks of haz_spectacular (seed-free): the slime under the door is defeated on 164
# (its pool drains on 164..223), sticker 0 is provoked on 373 and blasts on 397, the stick lands on 522
# and blasts on 634 (it chains sticker 1, which blasts on 642). The juicer plan closes the gate on tick 21
# and the Juicer crashes into the left wall on 139.
stills=(
  hazards:haz_spectacular:acid_full:120
  hazards:haz_spectacular:acid_drain:190
  hazards:haz_spectacular:rot_ring:385
  hazards:haz_spectacular:rot_ring_hot:394
  hazards:haz_spectacular:rot_wave_reach:404
  hazards:haz_spectacular:dyn_ring:560
  hazards:haz_spectacular:dyn_ring_hot:630
  hazards:haz_spectacular:dyn_wave_front:636
  hazards:haz_spectacular:dyn_wave_reach:641
  juicer:juicer:gate_raised:8
  juicer:juicer:gate_closed:30
  juicer:juicer:gate_crash:140
)
sizes=(640x360:1 1920x1080:3)
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
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    for still in "${stills[@]}"; do
      IFS=: read -r room plan name tick <<<"$still"
      dir="$caps/pass$pass-$res-$name"
      capture "$room" "$plan" "$tick:1:1" "$dir" --scale "$scale"
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      grep -q '^DANGER_FX sheets=7/7' "$dir/run.log" || { echo "the danger FX atlases did not load: $dir/run.log" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    IFS=: read -r room plan name tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no
    cmp -s "$a" "$b" && same=yes
    [[ "$same" == yes ]] || fail=1
    window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    match=no
    [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$match" == yes ]] || fail=1
    cp "$a" "$out/${name}_$res.png"
    echo "DANGER_FX_STILL room=$room plan=$plan name=$name tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
# The dynamite beat at game speed: the stick lands (522), its ring burns, turns hot, the 64 px wave,
# the chained sticker's ring and its 48 px wave (ticks 516..670, every 2 ticks).
dir="$caps/gif-dynamite"
capture hazards haz_spectacular 516:2:78 "$dir"
python3 scripts/room_gif.py "$dir" "$out/dynamite_beat_640x360.gif" --step-ticks 2
window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
match=no
[[ -n "$window" && "$window" == "$headless" ]] && match=yes
[[ "$match" == yes ]] || fail=1
echo "DANGER_FX_GIF room=hazards plan=haz_spectacular name=dynamite_beat ticks=516..670 step=2 window_hash=$window headless_hash=$headless hash_match=$match" >> "$out/capture_lines.txt"
(cd "$out" && sha256sum ./*.png ./*.gif ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
