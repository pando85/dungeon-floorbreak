#!/usr/bin/env bash
# DONUT-PROG-01 DP-F evidence: real Bevy 640x360 captures of the committed
# `donut_prog_*` render fixtures, joined into two GIFs and a contact sheet.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_dp_f_evidence.sh [out_dir]
#
# GIF 1  donut_prog_arena_ledge_base | donut_prog_arena_ledge_boost, ticks 30..100
#        every 2nd, side by side (2x crops): Donut runs into a 128 px ledge and
#        jumps once. Without the boost she peaks under the top and falls back;
#        with `jump_reach_boost` she lands on it (DP-Q14, DP-Q18).
# GIF 2  donut_prog_arena_blink_cornered, ticks 2..47 (3x crop): the AI Donut,
#        a goblin 28 px away, blinks out of its swing (press 20, arrival 26, the
#        swing on 31 finds no one) (DP-Q39 b).
# Sheet  the escape one tick at a time around the press and the arrival.
#
# Needs xvfb-run, ffmpeg, ImageMagick and a debug build (FLOORBREAK_BIN names the
# binary). Presentation evidence only: the captures are real renders, not
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
  # The first capture of a run is taken before the lighting settles (black): the
  # sequences below start one step early and drop it.
  rm -f "$work/$3"/frame_0000_*.png
}

tick_of() { basename "$1" .png | sed 's/.*_tick_0*//'; }

# crop scale label srcdir dstdir: crop, scale (nearest), caption with the tick.
caption() {
  local f t i=0
  mkdir -p "$work/$5"
  for f in "$work/$4"/frame_*.png; do
    t=$(tick_of "$f")
    magick "$f" -crop "$1" +repage -filter point -resize "$2" -gravity SouthWest -fill white \
      -undercolor '#000000b0' -pointsize 13 -annotate +4+4 " $3 tick $t " \
      "$work/$5/$(printf '%04d' "$i").png"
    i=$((i + 1))
  done
}

gif() { # fps name seqdir
  ffmpeg -y -loglevel error -framerate "$1" -i "$work/$3/%04d.png" \
    -vf 'split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none' \
    "$out/$2"
}

# --- GIF 1: the 128 px ledge, without and with the boost ---------------------
capture donut_prog_arena_ledge_base 28:2:37 ledge_base
capture donut_prog_arena_ledge_boost 28:2:37 ledge_boost
caption 340x230+0+90 200% "no boost:" ledge_base ledge_base_seq
caption 340x230+0+90 200% "jump_reach_boost:" ledge_boost ledge_boost_seq
mkdir -p "$work/ledge_pair"
for f in "$work"/ledge_base_seq/*.png; do
  n=$(basename "$f")
  magick "$f" "$work/ledge_boost_seq/$n" +append "$work/ledge_pair/$n"
done
gif 10 dp_f_gif1_ledge_128_side_by_side.gif ledge_pair

# --- GIF 2: the AI Donut blinks out of a swing -------------------------------
capture donut_prog_arena_blink_cornered 1:1:47 escape
caption 330x130+0+170 300% "" escape escape_seq
gif 10 dp_f_gif2_ai_escape_3x.gif escape_seq

# --- the contact sheet: press (20) and arrival (26), one tick at a time ------
capture donut_prog_arena_blink_cornered 17:1:13 sheet
i=0
for f in "$work/sheet"/frame_*.png; do
  t=$(tick_of "$f")
  magick "$f" -crop 330x130+0+170 +repage -scale 200% -gravity North -background '#14121c' \
    -fill white -pointsize 14 -splice 0x22 -annotate +0+3 "tick $t" "$work/sheet_$i.png"
  i=$((i + 1))
done
magick montage "$work"/sheet_{0..11}.png -tile 3x4 -geometry +4+4 -background '#14121c' \
  "$out/dp_f_escape_sheet.png"
