#!/usr/bin/env bash
# CAMERA-LAB: build the compact committed evidence set (docs/evidence/camera-lab)
# from the bulky capture directories of scripts/camera_lab_evidence.sh.
#   camera_lab_curate.sh SHOTS_DIR VIDEOS_DIR CROPS_DIR OUT_DIR
set -euo pipefail
shots="${1:?shots dir}"; videos="${2:?videos dir}"; crops="${3:?crops dir}"; out="${4:?out dir}"
mkdir -p "$out/sheets" "$out/frames" "$out/videos" "$out/crops"
names=("The First Rematch" "Hunters From Above" "Break A Breach" "The Hoarder's Lair" "The Juicer's Gym" "Ball of Swine")
covs=(1 1.25 1.5 1.75)
labels=("1.00x 640x360" "1.25x 800x450" "1.50x 960x540" "1.75x 1120x630")
# One contact sheet per room: top row solo scripted action, bottom row 2P
# separated, one column per fixed coverage (1920x1080 outputs, each cell 480x270).
for room in 1 2 3 4 5 6; do
  cells=()
  for kind in solo sep; do
    for i in 0 1 2 3; do
      cells+=(-label "R${room} ${kind} ${labels[$i]}" "$shots/R${room}_${kind}_${covs[$i]}_1920x1080.png")
    done
  done
  montage "${cells[@]}" -tile 4x2 -geometry 480x270+3+3 -background '#15121c' -fill white -pointsize 14 \
    -quality 88 "$out/sheets/room0${room}_${names[$((room-1))]// /_}.jpg" 2>/dev/null || \
  montage "${cells[@]}" -tile 4x2 -geometry 480x270+3+3 -background '#15121c' -quality 88 "$out/sheets/room0${room}.jpg"
done
# Full resolution frames of the headline comparisons.
cp "$shots/R1_solo_1_1920x1080.png"      "$out/frames/room01_solo_1.0x_1920x1080.png"
cp "$shots/R1_solo_1.5_1920x1080.png"    "$out/frames/room01_solo_1.5x_1920x1080.png"
cp "$shots/R6_sep_1.5_1920x1080.png"     "$out/frames/room06_2p_separated_1.5x_1920x1080.png"
cp "$shots/R6_sep_auto_1920x1080.png"    "$out/frames/room06_2p_separated_auto_1920x1080.png"
cp "$shots/R5_sep_1_1280x800.png"        "$out/frames/room05_2p_separated_1.0x_1280x800_deck.png"
cp "$shots/R5_sep_1.5_1280x800.png"      "$out/frames/room05_2p_separated_1.5x_1280x800_deck.png"
cp "$shots/R4_solo_1.5_1920x1080.png"    "$out/frames/room04_solo_1.5x_1920x1080.png"
# A 4K frame, cropped to its centre 1920x1080 so the repository stays small.
magick "$shots/R3_solo_1.5_3840x2160.png" -gravity center -crop 1920x1080+0+0 +repage "$out/frames/room03_solo_1.5x_3840x2160_centre_crop.png"
# Pixel-sampling crops: Carl and Donut at the spawn, 1080p, every coverage, 3x nearest.
for c in 1 1.25 1.5 1.75; do
  k=$(python3 -c "print(1920/{1:640,1.25:800,1.5:960,1.75:1120}[$c])")
  x=$(python3 -c "print(max(0,int(125*$k-60)))"); y=$(python3 -c "print(max(0,int(0.8*1080-110)))")
  magick "$crops/crop_$c.png" -crop 300x130+${x}+${y} +repage -filter point -resize 300% "$out/crops/spawn_${c}x_1080p_crop_x3.png"
done
montage -label "1.00x: 3 px texels (exact)" "$out/crops/spawn_1x_1080p_crop_x3.png" \
  -label "1.25x: 2.4 px texels (2 or 3)" "$out/crops/spawn_1.25x_1080p_crop_x3.png" \
  -label "1.50x: 2 px texels (exact)" "$out/crops/spawn_1.5x_1080p_crop_x3.png" \
  -label "1.75x: 1.71 px texels (1 or 2)" "$out/crops/spawn_1.75x_1080p_crop_x3.png" \
  -tile 2x2 -geometry +2+2 -background '#15121c' -fill white -pointsize 14 "$out/crops/spawn_crops_1080p.png"
rm -f "$out"/crops/spawn_*_crop_x3.png
# Videos (re-encoded small) and the frame-time table.
cp "$videos"/*.mp4 "$out/videos/"
echo CURATE_OK
