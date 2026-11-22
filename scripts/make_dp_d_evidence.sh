#!/usr/bin/env bash
# DONUT-PROG-01 DP-D evidence: real Bevy 640x360 (1x) captures of the committed
# `donut_prog_*` render fixtures, joined into GIFs (1x full frame, and 3x
# nearest-neighbour crops around Donut and her plate) and one contact sheet.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_dp_d_evidence.sh [out_dir]
#
# GIF 1  donut_prog_grants_2p: Donut's two grants on one tick, the two notices
#        queued (LIGHT FEET, then PUDDLE JUMPER) and her socket turning from the
#        padlock to the equipped chip.
# GIF 2  donut_prog_arena_blink_tour 0..720: the flat blink, the socket cooling
#        over 9 s (the underline shrinks), the ready flash, the wall blink.
# GIF 3  the flat blink tick by tick (3x): crouch, ripple, streak, destination.
# GIF 4  the wall blink (3x): she stops in front of the parapet.
# GIF 5  donut_prog_blink_fizzle: the arrival fizzle and its shrinking ring (3x).
# GIF 6  donut_prog_arena_blink_edge: the no-mana and no-destination refusals and
#        the socket flash (3x of her plate).
# Sheet  the blink cues one tick at a time (dp_d_cues_sheet.png).
#
# Needs xvfb-run, ffmpeg, ImageMagick and a debug build (FLOORBREAK_BIN names the
# binary). Presentation evidence only; the captures are real renders, not
# reproduced bit for bit by CI (driver dependent). The deterministic half (state
# hashes, event ticks) is pinned by the tests and the committed headless replays.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export FLOORBREAK_RECORD=off
out="${1:-docs/evidence/donut-prog-01}"
bin="${FLOORBREAK_BIN:-./target/debug/floor01_scene}"
work="$(mktemp -d -p "${TMPDIR:-/tmp}")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

capture() { # fixture first:step:count dir
  mkdir -p "$work/$3"
  xvfb-run -a -s '-screen 0 1920x1200x24' "$bin" --scale 1 --party-fixture "$1" \
    --capture-sequence "$2" --capture-dir "$work/$3" >"$work/$3.log" 2>&1
  # The first capture of a run is taken before the lighting settles (black).
  rm -f "$work/$3"/frame_0000_*.png
}

# crop scale first_tick step label dir seqdir: crop, scale (nearest), tick caption,
# appended to the numbered frame list of `seqdir`.
caption() {
  local i=0 f n
  mkdir -p "$work/$7"
  for f in "$work/$6"/frame_*.png; do
    n=$(ls "$work/$7" | wc -l)
    magick "$f" -crop "$1" +repage -filter point -resize "$2" -gravity SouthWest -fill white \
      -undercolor '#000000b0' -pointsize 13 -annotate +4+4 " $5 tick $(( $3 + $4 * i )) " \
      "$work/$7/$(printf '%04d' "$n").png"
    i=$((i + 1))
  done
}

gif() { # fps name seqdir
  ffmpeg -y -loglevel error -framerate "$1" -i "$work/$3/%04d.png" \
    -vf 'split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none' \
    "$out/$2"
}

# --- GIF 1: the grants -----------------------------------------------------
capture donut_prog_grants_2p 240:6:50 grants
caption 640x360+0+0 100% 240 6 "" grants grantsseq
gif 10 dp_d_gif1_grants.gif grantsseq

# --- GIF 2: the socket cycle over 9 s (every 6th tick) ---------------------
capture donut_prog_arena_blink_tour 6:6:119 cycle
caption 640x360+0+0 100% 6 6 "" cycle cycleseq
gif 20 dp_d_gif2_cycle_1x.gif cycleseq

# --- GIF 3: the flat blink ---------------------------------------------------
capture donut_prog_arena_blink_tour 36:1:20 flat
caption 300x120+170+190 300% 36 1 "" flat flatseq
gif 10 dp_d_gif3_flat_3x.gif flatseq

# --- GIF 4: the wall ---------------------------------------------------------
capture donut_prog_arena_blink_tour 596:1:20 wall
caption 300x120+260+190 300% 596 1 "" wall wallseq
gif 10 dp_d_gif4_wall_3x.gif wallseq

# --- GIF 5: the fizzle -------------------------------------------------------
capture donut_prog_blink_fizzle 132:1:24 fizzle
caption 640x360+0+0 100% 132 1 "" fizzle fizzleseq
gif 10 dp_d_gif5_fizzle_1x.gif fizzleseq

# --- GIF 6: the refusals (her plate, 3x) -------------------------------------
capture donut_prog_arena_blink_edge 266:1:14 edge_a
capture donut_prog_arena_blink_edge 416:1:14 edge_b
caption 200x56+440+4 300% 266 1 "no mana:" edge_a edgeseq
caption 200x56+440+4 300% 416 1 "no destination:" edge_b edgeseq
gif 10 dp_d_gif6_refusals_3x.gif edgeseq

# --- the contact sheet: the blink one tick at a time, 3x ---------------------
capture donut_prog_arena_blink_tour 38:1:12 cues
panel() { # index label
  local f
  f=$(ls "$work/cues"/frame_*.png | sed -n "$1p")
  magick "$f" -crop 220x110+260+200 +repage -scale 300% -gravity North -background '#14121c' \
    -fill white -pointsize 14 -splice 0x22 -annotate +0+3 "$2" "$work/panel_$1.png"
}
labels=("tick 39: ready" "tick 40: press (t0)" "tick 41" "tick 42" "tick 43" "tick 44" "tick 45 (t5)" "tick 46: arrival (t6)" "tick 47: recovery" "tick 48" "tick 49")
for i in $(seq 1 11); do panel "$i" "${labels[$((i - 1))]}"; done
magick montage "$work"/panel_{1..11}.png -tile 4x3 -geometry +4+4 -background '#14121c' \
  "$out/dp_d_cues_sheet.png"
