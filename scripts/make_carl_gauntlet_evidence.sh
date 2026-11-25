#!/usr/bin/env bash
# CARL-GAUNTLET-01 evidence: the art sheets (python) and the in-room Bevy captures of rooms 5 and 6.
#
#   bash scripts/make_carl_gauntlet_evidence.sh [OUT_DIR]       (default docs/evidence/carl-gauntlet-01)
#
# Needs the exported atlases (bash scripts/export_assets.sh, which writes the *_gauntlet atlases too), a built
# floor01_scene (BIN, default ./target/debug/floor01_scene), xvfb-run and flock. Every still is one
# scripts/room_capture.sh run: a real window under xvfb (the Bevy presentation) and the headless run to the same
# tick; the two final COMBAT_STATE hashes must be equal (the overlay is presentation only), or the script fails.
#   Room 5 (juicer pilot): the defeat and the grant of fist_reinforced are tick 3175. Pre-grant still: tick 3165;
#   post-grant stills: ticks 3200 and 3230.
#   Room 6 (mastery:carl_solo:2): the room's pin holds every capability, so the overlay is on from tick 0. Carl's
#   haymaker (move punch_b) starts on tick 510 (contact about 515); the GIF is ticks 498..542.
# Sizes: 640 x 360 and 640 x 400 (--deck), at scale 1 and 3.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
out="${1:-docs/evidence/carl-gauntlet-01}"
caps="${CAPS:-artifacts/carl-gauntlet-captures}"
case "$(realpath -m -- "$caps")" in "$(pwd)"/artifacts/?*) ;; *) echo "CAPS must be inside ./artifacts/" >&2; exit 2;; esac
mkdir -p "$out"
rm -rf "$caps"
mkdir -p "$caps"
python3 scripts/make_carl_gauntlet_evidence.py "$out" > /dev/null
: > "$out/capture_lines.txt"
# name room plan tick
stills=(
  "room5_pre_grant juicer juicer 3165"
  "room5_post_grant juicer juicer 3200"
  "room5_post_grant_exit juicer juicer 3230"
  "room6_haymaker mastery mastery:carl_solo:2 516"
  "room6_run mastery mastery:carl_solo:2 150"
)
# res scale extra-flag
sizes=("640x360 1 " "1920x1080 3 " "640x400 1 --deck" "1920x1200 3 --deck")
for still in "${stills[@]}"; do
  read -r name room plan tick <<<"$still"
  for size in "${sizes[@]}"; do
    read -r res scale extra <<<"$size"
    dir="$caps/$name-$res"
    for attempt in 1 2 3; do
      # shellcheck disable=SC2086
      bash scripts/room_capture.sh "$room" "$plan" "$tick:1:1" "$dir" --scale "$scale" $extra > "$dir.txt" 2> "$dir.err" && break
      [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
    done
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
    cp "$frame" "$out/${name}_${res}.png"
    sed "s/^/GAUNTLET_STILL name=$name res=$res /" "$dir.txt" >> "$out/capture_lines.txt"
  done
done
# the haymaker GIF: ticks 498..542, 2 ticks a frame (33 ms), at scale 3, cropped to the fight and enlarged by nothing more
bash scripts/room_capture.sh mastery mastery:carl_solo:2 498:2:23 "$caps/haymaker-3x" --scale 3 > "$caps/haymaker-3x.txt"
sed "s/^/GAUNTLET_GIF name=room6_haymaker_3x /" "$caps/haymaker-3x.txt" >> "$out/capture_lines.txt"
python3 scripts/room_gif.py "$caps/haymaker-3x" "$out/gif_room6_haymaker_3x.gif" --step-ticks 2 --crop "${GIF_CROP:-720,660,1200,960}" --zoom 1
if grep -v ' match=yes$' "$out/capture_lines.txt" | grep -q .; then echo "a capture's window hash differs from its headless hash" >&2; exit 1; fi
echo "wrote $(wc -l < "$out/capture_lines.txt") capture lines into $out"
