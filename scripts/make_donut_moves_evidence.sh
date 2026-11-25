#!/usr/bin/env bash
# DONUT-MOVES-ART-01 evidence: real Bevy captures (640x360 at 1x and 1920x1080 at 3x) of the committed render
# fixtures, with the production cue art drawn by the presenter, joined into one real-render GIF and in-scene stills.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_donut_moves_evidence.sh [out_dir]
#
# scene_blink_tour_1x.gif        donut_prog_arena_blink_tour 38..61: the flat blink at 60 Hz (press at tick 40, arrival
#                                at 46): crouch, dip, vanish, the streak, the pop, the rise, the settle (crop of
#                                the 640x360 frame, nearest neighbour 2x)
# scene_blink_arrival_{1x,3x}.png  the same fixture on tick 48 (blink_arrive): the full 2P frame at 1x and at 3x
# scene_blink_fizzle_{1x,3x}.png   donut_prog_blink_fizzle on tick 138 (the bonk)
# scene_evade_{1x,3x}.png          cf02_dm_reflex_cues on tick 733 (evade_twist at the arc peak)
#
# Needs xvfb-run, ffmpeg, ImageMagick and a debug build (FLOORBREAK_BIN names the binary). Presentation evidence
# only; the captures are real renders, not reproduced bit for bit by CI (driver dependent). The deterministic half
# (state hashes, event ticks) is pinned by the tests and the committed headless replays.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export FLOORBREAK_RECORD=off
out="${1:-docs/evidence/donut-moves-art-01}"
bin="${FLOORBREAK_BIN:-./target/debug/floor01_scene}"
work="$(mktemp -d -p "${TMPDIR:-/tmp}")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

capture() { # fixture first:step:count scale dir
  mkdir -p "$work/$4"
  xvfb-run -a -s '-screen 0 1920x1200x24' "$bin" --scale "$3" --party-fixture "$1" \
    --capture-sequence "$2" --capture-dir "$work/$4" >"$work/$4.log" 2>&1
  # The first capture of a run is taken before the lighting settles (black).
  rm -f "$work/$4"/frame_0000_*.png
}

still() { # fixture tick scale name
  capture "$1" "$(( $2 - 1 )):1:2" "$3" "still_$4_$3"
  cp "$work/still_$4_$3"/frame_0001_*.png "$out/scene_$4_$(( $3 ))x.png"
}

# --- the blink, real render, 60 Hz --------------------------------------------------------------------------
capture donut_prog_arena_blink_tour 38:1:24 1 tour
i=0
mkdir -p "$work/tourseq"
for f in "$work/tour"/frame_*.png; do
  magick "$f" -crop 260x110+190+200 +repage -filter point -resize 200% -gravity SouthWest -fill white \
    -undercolor '#000000b0' -pointsize 13 -annotate +4+4 " tick $(( 39 + i )) " \
    "$work/tourseq/$(printf '%04d' "$i").png"
  i=$((i + 1))
done
ffmpeg -y -loglevel error -framerate 30 -i "$work/tourseq/%04d.png" \
  -vf 'split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none' \
  "$out/scene_blink_tour_1x.gif"

for scale in 1 3; do
  still donut_prog_arena_blink_tour 48 "$scale" blink_arrival
  still donut_prog_blink_fizzle 138 "$scale" blink_fizzle
  still cf02_dm_reflex_cues 733 "$scale" evade
done
echo "wrote the scene captures into $out"
