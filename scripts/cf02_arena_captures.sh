#!/usr/bin/env bash
# CF02-F evidence: real Bevy captures of the microarena (`floor01_scene --cf02`) at
# 640x360, 640x400 and 1920x1080, and 1x GIFs of four beats (Donut's bail-out, a shore
# drop, a goblin self-own, a perch dive), taken in the real render loop.
#
#   xvfb-run -a -s '-screen 0 1920x1200x24' bash scripts/cf02_arena_captures.sh [out_dir]
#
# Needs a debug build (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh) and ImageMagick. Run from the repository root. The
# images show the state at fixed ticks (the deterministic autopilot or a committed
# fixture script drives them); they are evidence of what was drawn, not a game-feel
# verdict, and they are not reproduced bit for bit by CI (driver dependent).
set -euo pipefail
bin=./target/debug/floor01_scene
out="${1:-docs/evidence/cf02/cf02-c2f}"
mkdir -p "$out/captures" "$out/gifs"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# 1. The room at three resolutions: the live flag, the autopilot, ticks 40 (the spawn
#    end: left wall and shore) and 330 (the first wave at the perch and the parapet).
for spec in 360:--scale\ 1 400:--deck\ --scale\ 1 1080:--scale\ 3; do
  label="${spec%%:*}"
  flags="${spec#*:}"
  case "$label" in 360) res=640x360 ;; 400) res=640x400 ;; 1080) res=1920x1080 ;; esac
  for tick in 40 330; do
    dir="$work/${label}_${tick}"
    # shellcheck disable=SC2086
    "$bin" --cf02 --companion off --autopilot $flags \
      --capture-sequence "$((tick - 1)):1:2" --capture-dir "$dir" > "$work/log.txt" 2>&1
    grep -q 'FLOOR1_CAPTURE_SEQUENCE_DONE count=2' "$work/log.txt"
    frame="$(find "$dir" -name 'frame_0001_*.png')"
    cp "$frame" "$out/captures/arena_t${tick}_${res}.png"
    grep '^COMBAT_STATE' "$work/log.txt" | tail -n 1 > "$out/captures/state_arena_t${tick}_${res}.txt"
    [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res" >&2; exit 1; }
  done
done

# 2. 1x GIFs: name:fixture:first:step:count. Every frame is one real render of the
#    committed replay of the fixture; a frame every second tick is 30 frames a second.
gifs=(
  donut_bailout:cf02_donut_bailout:4:2:25
  shore_drop:cf02_block_hurts_carl:60:2:30
  goblin_self_own:cf02_goblin_self_own:30:2:40
  perch_dive:cf02_perch_dive:64:2:24
)
for spec in "${gifs[@]}"; do
  IFS=: read -r name fixture first step count <<<"$spec"
  dir="$work/$name"
  "$bin" --scale 1 --party-fixture "$fixture" \
    --capture-sequence "${first}:${step}:${count}" --capture-dir "$dir" > "$work/log.txt" 2>&1
  grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${count}" "$work/log.txt"
  # The first frame of a run is often unlit (warm-up): drop it by capturing from
  # one tick earlier is not needed here, the settle frames of the loop cover it.
  magick -delay 3 -loop 0 "$dir"/frame_*.png -layers Optimize "$out/gifs/${name}_1x.gif"
  grep '^COMBAT_STATE' "$work/log.txt" | tail -n 1 > "$out/gifs/state_${name}.txt"
done
( cd "$out" && find captures gifs -type f \( -name '*.png' -o -name '*.gif' \) | sort | xargs sha256sum > SHA256SUMS )
echo "CF02_ARENA_CAPTURES_DONE"
