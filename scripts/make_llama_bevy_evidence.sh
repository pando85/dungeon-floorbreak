#!/usr/bin/env bash
# DEMO-01 PR B evidence: real 1920x1080 Bevy captures of the Bad Llama (spit
# tell and glob, the slam, the plain down and the crush under the stone) into
# docs/evidence/llama-bevy. Needs the debug binary (cargo build --locked --bin
# floor01_scene), the exported atlases (scripts/export_assets.sh), xvfb-run and
# ImageMagick. The README is hand-written.
#
#   bash scripts/make_llama_bevy_evidence.sh
#
# The captures are the real render loop under xvfb (software GL on the CI and
# development machines): no game-feel claim, no frame-time claim.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${LLAMA_BIN:-./target/debug/floor01_scene}"
tmp=artifacts/llama-bevy
dest=docs/evidence/llama-bevy
rm -rf "$tmp"
mkdir -p "$tmp" "$dest"

# cap NAME FIXTURE PRESET FIRST COUNT: one real 1920x1080 frame per tick.
cap() {
  local name="$1" fixture="$2" preset="$3" first="$4" count="$5"
  xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --scale 3 --fixture "$fixture" \
    --gore-preset "$preset" --capture-sequence "${first}:1:${count}" \
    --capture-dir "$tmp/$name" > "$tmp/$name.log" 2>&1
  grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${count}" "$tmp/$name.log"
  # The state the capture ends on must be the headless replay's (same hash).
  local last=$((first + count - 1))
  diff <(grep '^COMBAT_STATE' "$tmp/$name.log" | tail -n 1) \
    <("$bin" --headless-floor01-smoke --fixture "$fixture" --capture-tick "$last" | grep '^COMBAT_STATE' | tail -n 1)
}

frame() { ls "$tmp/$1"/frame_*_tick_$(printf '%05d' "$2").png; }

cap spit cp05_llama_spit 4 30 61
cap slam_p4 cp05_llama_slam 4 158 50
cap slam_p1 cp05_llama_slam 1 158 50
cap down_p4 cp05_llama_down 4 58 50
cap down_p1 cp05_llama_down 1 58 50
cap crush_p4 cp05_llama_crush 4 136 30

# Stills (1920x1080, as captured).
still() { cp "$(frame "$1" "$2")" "$dest/$3"; }
still spit 30 still_spit_tell_glow_tick030_1920x1080.png
still spit 37 still_spit_release_tick037_1920x1080.png
still spit 55 still_spit_glob_in_flight_tick055_1920x1080.png
still spit 77 still_spit_hits_carl_tick077_1920x1080.png
still slam_p4 164 still_slam_p4_contact_tick164_1920x1080.png
still slam_p4 190 still_slam_p4_broken_heap_tick190_1920x1080.png
still slam_p1 190 still_slam_p1_broken_heap_tick190_1920x1080.png
still down_p4 100 still_down_p4_tick100_1920x1080.png
still down_p1 100 still_down_p1_tick100_1920x1080.png
still crush_p4 139 still_crush_p4_stone_over_llama_tick139_1920x1080.png
still crush_p4 142 still_crush_p4_flatten_tick142_1920x1080.png
still crush_p4 150 still_crush_p4_remains_tick150_1920x1080.png

# Native 640x360 captures (--scale 1, the virtual resolution itself).
cap1x() {
  local name="$1" fixture="$2" preset="$3" tick="$4" out="$5"
  xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --scale 1 --fixture "$fixture" \
    --gore-preset "$preset" --capture-sequence "${tick}:1:1" \
    --capture-dir "$tmp/$name" > "$tmp/$name.log" 2>&1
  grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=1" "$tmp/$name.log"
  cp "$(frame "$name" "$tick")" "$dest/$out"
  [[ "$(identify -format '%wx%h' "$dest/$out")" == "640x360" ]]
}
cap1x n_spit cp05_llama_spit 4 55 still_spit_glob_tick055_640x360.png
cap1x n_slam cp05_llama_slam 4 190 still_slam_p4_broken_heap_tick190_640x360.png
cap1x n_down cp05_llama_down 4 100 still_down_p4_tick100_640x360.png
cap1x n_crush cp05_llama_crush 4 139 still_crush_p4_stone_over_llama_tick139_640x360.png

# Contact sheets of the same frames, 640x360 per cell (3x down), labelled.
sheet() {
  local out="$1"; shift
  local args=()
  while (($#)); do
    args+=(-label "$2" "$(frame "$1" "$3")")
    shift 3
  done
  montage "${args[@]}" -geometry 640x360+4+4 -tile 3x -background '#101010' -fill white \
    -pointsize 18 -depth 8 -define png:compression-level=9 "$dest/$out"
}
sheet sheet_spit_p4_ticks30-77.png \
  spit "tell glow t30" 30 spit "release t37" 37 spit "glob t45" 45 \
  spit "glob t55" 55 spit "glob t70" 70 spit "hit t77" 77
sheet sheet_slam_p4_vs_p1.png \
  slam_p4 "p4 contact t164" 164 slam_p4 "p4 t170" 170 slam_p4 "p4 heap t190" 190 \
  slam_p1 "p1 contact t164" 164 slam_p1 "p1 t170" 170 slam_p1 "p1 heap t190" 190
sheet sheet_down_p4_vs_p1.png \
  down_p4 "p4 kill t63" 63 down_p4 "p4 t70" 70 down_p4 "p4 t100" 100 \
  down_p1 "p1 kill t63" 63 down_p1 "p1 t70" 70 down_p1 "p1 t100" 100
sheet sheet_crush_p4_stone_over_llama.png \
  crush_p4 "landing t139" 139 crush_p4 "flatten t142" 142 crush_p4 "remains t146" 146 \
  crush_p4 "t150" 150 crush_p4 "t160" 160 crush_p4 "t165" 165

# GIFs at 960x540, every second tick (a 33 ms step, 30 fps).
gif() {
  local out="$1" name="$2" first="$3" last="$4" list=()
  for ((t = first; t <= last; t += 2)); do list+=("$(frame "$name" "$t")"); done
  convert -delay 3 -loop 0 -resize 960x540 "${list[@]}" -layers Optimize -colors 96 "$dest/$out"
}
gif spit_p4_ticks30-90_960x540.gif spit 30 90
gif slam_p4_ticks158-207_960x540.gif slam_p4 158 206
gif slam_p1_ticks158-207_960x540.gif slam_p1 158 206
gif down_p4_ticks58-107_960x540.gif down_p4 58 106
gif crush_p4_ticks136-165_960x540.gif crush_p4 136 164

# The headless logs of the five fixtures and the checksums.
for f in cp05_llama_slam cp05_llama_down cp05_llama_spit cp05_llama_offscreen cp05_llama_crush; do
  cp "docs/evidence/cp01-feel/headless_${f}.txt" "$dest/headless_${f}.txt"
done
(cd "$dest" && sha256sum -- *.png *.gif *.txt > SHA256SUMS)
echo "LLAMA_BEVY_EVIDENCE_OK: $(ls "$dest" | wc -l) files in $dest"
