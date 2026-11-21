#!/usr/bin/env bash
# DONUT-MOBILITY-01 DM-C evidence: real Bevy 640x360 (1x) captures of the
# committed fixtures, joined into GIFs at 2x (nearest neighbour) and a contact
# sheet of the reflex cues at 3x.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_dm_c_evidence.sh [out_dir]
#
# GIF 1 and 2 of the work order (the run race and the jump race) are the DM-A
# lane GIFs, `dm_a_run_race_1x.gif` and `dm_a_jump_race_1x.gif`: those fixtures
# have no biscuit, so no cue, and DM-C changes nothing in them.
# GIF 3  cf02_dm_reflex_cues: glint while armed, the swing, the evade, smug, the
#        glint gone (ticks 714..790), then, 12 s later, armed again (1446..1486).
# GIF 4  cf02_arena_dm_perch_jump: the microarena with the HUD drawn, her jump
#        from the perch under the lane ceiling (DM-Q27).
# Sheet  the stand-in cues one tick at a time (dm_c_cues_sheet.png).
#
# Needs xvfb-run, ffmpeg, ImageMagick and a debug build (FLOORBREAK_BIN names the
# binary). Presentation evidence only; the captures are real renders, not
# reproduced bit for bit by CI (driver dependent). The deterministic half (state
# hashes, event ticks) is pinned by the tests and the committed headless replays.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export FLOORBREAK_RECORD=off
out="${1:-docs/evidence/donut-mobility-01}"
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

# caption dir crop first_tick step label seqdir: crop, 2x nearest, tick caption,
# appended to the numbered frame list of `seqdir`.
caption() {
  local i=0 f n
  mkdir -p "$work/$6"
  for f in "$work/$1"/frame_*.png; do
    n=$(ls "$work/$6" | wc -l)
    magick "$f" -crop "$2" +repage -filter point -resize 200% -gravity SouthWest -fill white \
      -undercolor '#000000b0' -pointsize 13 -annotate +4+4 " $5 tick $(( $3 + $4 * i )) " \
      "$work/$6/$(printf '%04d' "$n").png"
    i=$((i + 1))
  done
}

gif() { # fps name seqdir
  ffmpeg -y -loglevel error -framerate "$1" -i "$work/$3/%04d.png" \
    -vf 'split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none' \
    "$out/$2"
}

# --- GIF 3: the reflex -------------------------------------------------------
capture cf02_dm_reflex_cues 712:2:40 reflex_a
capture cf02_dm_reflex_cues 1444:2:22 reflex_b
caption reflex_a 288x108+250+190 714 2 "" reflex
caption reflex_b 288x108+250+190 1446 2 "12 s later:" reflex
# Half speed: the cues last a few ticks each.
gif 15 dm_c_gif3_reflex.gif reflex

# --- GIF 4: the perch jump under the HUD ------------------------------------
capture cf02_arena_dm_perch_jump 68:2:76 perch
caption perch 640x230+0+0 70 2 "" perchseq
gif 30 dm_c_gif4_perch_jump_hud.gif perchseq

# --- the contact sheet: one tick per panel, 3x, around her ------------------
capture cf02_dm_reflex_cues 698:1:100 cues
panel() { # tick crop label
  local n=$(( $1 - 698 ))
  magick "$work/cues/$(printf 'frame_%04d_tick_%05d.png' "$n" "$1")" -crop "$2" +repage \
    -scale 300% -gravity North -background '#14121c' -fill white -pointsize 14 \
    -splice 0x22 -annotate +0+3 "$3" "$work/panel_$1.png"
}
capture cf02_dm_reflex_cues 1448:1:6 cues_b
magick "$work/cues_b/$(ls "$work/cues_b" | sed -n 2p)" -crop 120x70+295+235 +repage \
  -scale 300% -gravity North -background '#14121c' -fill white -pointsize 14 \
  -splice 0x22 -annotate +0+3 "tick 1450: armed again, glint on" "$work/panel_1450.png"
panel 710 120x70+377+235 "tick 710: not armed (no cue)"
panel 721 120x70+377+235 "tick 721: armed, glint + tail flick"
panel 727 120x70+377+235 "tick 727: glint off (26 ticks)"
panel 729 120x70+377+235 "tick 729: evade, age 0, streak"
panel 732 120x70+377+235 "tick 732: age 3, afterimages"
panel 735 120x70+377+235 "tick 735: age 6, arc peak 10 px"
panel 740 120x70+377+235 "tick 740: age 11, last tick"
panel 762 120x70+270+235 "tick 762: smug (CF02-I overlay)"
panel 790 120x70+270+235 "tick 790: spent, no glint"
magick montage "$work"/panel_710.png "$work"/panel_721.png "$work"/panel_727.png \
  "$work"/panel_729.png "$work"/panel_732.png "$work"/panel_735.png "$work"/panel_740.png \
  "$work"/panel_762.png "$work"/panel_790.png "$work"/panel_1450.png \
  -tile 5x2 -geometry +4+4 -background '#14121c' "$out/dm_c_cues_sheet.png"
