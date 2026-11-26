#!/usr/bin/env bash
# F01-SIM-TICKS: the in-scene evidence of the two tick-driven effects (spec:
# docs/02-art/f01-sim-ticks-spec.md):
#   - the Boiler weight's landing dust (room 2, `chain` plan: the weight lands on tick 695);
#   - the room 5 gate drop and its slam dust (`juicer` plan: the gate closes on tick 21, the grate
#     hits the floor on 29, the dust plays on 29..48).
# scripts/room_capture.sh renders the real Bevy window under xvfb at gameplay scale (640x360,
# `--scale 1`), one frame per tick around each tick, and checks that the window's final COMBAT_STATE
# hash equals the headless run's (the presentation is read-only). Every sequence is captured TWICE
# and every frame of the two passes must be byte-identical. Output in $OUT: the selected stills, one
# game-speed GIF per effect (every tick), capture_lines.txt and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/f01-sim-ticks/scene)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-sim-ticks/scene}"
caps="${CAPTURES:-artifacts/sim-ticks}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
for sheet in gate slam-dust; do
  [[ -f "generated/art/f01-dfx-$sheet.png" ]] || { echo "generated/art/f01-dfx-$sheet.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
done
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:room:plan:first:count (one frame per tick) and the stills taken from each sequence.
sequences=(
  boiler_landing:boiler:chain:693:24
  gate_drop:juicer:juicer:19:32
)
stills_boiler_landing=(694 695 697 700 704 710 715 716)
stills_gate_drop=(20 21 23 25 27 29 31 34 38 44 49)
capture() { # room plan seq dir
  # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the
  # machine, not the run, so a capture is retried up to three times. A hash mismatch is not retried
  # away: it fails the checks below.
  for attempt in 1 2 3; do
    bash scripts/room_capture.sh "$1" "$2" "$3" "$4" --scale 1 > "$4.txt" 2> "$4.err" && return 0
    [[ "$attempt" -lt 3 ]] || { echo "capture $4 failed 3 times: see $4/run.log" >&2; exit 1; }
  done
}
for pass in 1 2; do
  for s in "${sequences[@]}"; do
    IFS=: read -r name room plan first count <<<"$s"
    dir="$caps/pass$pass-$name"
    capture "$room" "$plan" "$first:1:$count" "$dir"
    grep -q '^DANGER_FX sheets=8/8' "$dir/run.log" || { echo "the danger FX atlases did not load: $dir/run.log" >&2; exit 1; }
    [[ "$(find "$dir" -name 'frame_*.png' | wc -l)" -eq "$count" ]] || { echo "missing frames in $dir" >&2; exit 1; }
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for s in "${sequences[@]}"; do
  IFS=: read -r name room plan first count <<<"$s"
  a="$caps/pass1-$name" b="$caps/pass2-$name"
  same=yes
  for f in "$a"/frame_*.png; do
    cmp -s "$f" "$b/$(basename "$f")" || same=no
  done
  [[ "$same" == yes ]] || fail=1
  window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$a.txt")"
  headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$a.txt")"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$match" == yes ]] || fail=1
  last=$((first + count - 1))
  echo "SIM_TICKS_SEQUENCE room=$room plan=$plan name=$name ticks=$first..$last step=1 size=640x360 window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same" >> "$out/capture_lines.txt"
  declare -n stills="stills_$name"
  for tick in "${stills[@]}"; do
    frame="$(find "$a" -name "frame_*_tick_$(printf '%05d' "$tick").png")"
    [[ -n "$frame" ]] || { echo "no frame for tick $tick in $a" >&2; exit 1; }
    [[ "$(identify -format '%wx%h' "$frame")" == 640x360 ]] || { echo "bad size: $frame" >&2; exit 1; }
    cp "$frame" "$out/${name}_t${tick}_640x360.png"
    echo "SIM_TICKS_STILL room=$room plan=$plan name=$name tick=$tick size=640x360 sha256=$(sha256sum "$frame" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
  unset -n stills
  python3 scripts/room_gif.py "$a" "$out/${name}_640x360.gif" --step-ticks 1
done
(cd "$out" && sha256sum ./*.png ./*.gif ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
