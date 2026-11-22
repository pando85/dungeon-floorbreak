#!/usr/bin/env bash
# PT-GATE-E evidence (plan section 10, item 7b), into docs/evidence/f01-playtest/: room 4's tactical and
# direct routes, each at 640x360 and 640x400 (`--deck`), generated twice and compared byte for byte.
#   tactical: `--room-pilot haz_tactical` (HAZ-07 pattern 2: R1 spent by bait and retreat, then R2 baited
#             over the slime S2 so that its blast hits S2), ticks 340 to 700, 6 ticks a frame;
#   direct:   `--room-pilot hazards:carl_solo:0` (the aligned HAZ-06 melee route: the PT-TUNE pilot that does
#             not dodge hazards; the library fixture `haz_room_solo_carl` at seed 0), ticks 320 to 650.
# Each window run is compared with the headless run and with the timed live loop (`--exit-after-ticks`);
# the lines go to capture_lines_e.txt. The spectacular route is scripts/make_f01_gate_d_evidence.sh; the
# render-loop replay of the aligned library pilots is scripts/make_f01_gate_d_replay.sh.
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run and Pillow. Runs the windows one after the other.
#   BIN=<floor01_scene>   CAPTURES: a scratch dir under artifacts/ (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/f01-gate-e-captures}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
out=docs/evidence/f01-playtest
rm -rf "$caps"
mkdir -p "$caps" "$out"
rm -f "$out"/route-room4-tactical*.gif "$out"/route-room4-direct*.gif "$out/capture_lines_e.txt"
: > "$out/capture_lines_e.txt"
# route name, plan, first:step:count, ticks per frame
routes=(
  "tactical haz_tactical 340:6:61 6"
  "direct hazards:carl_solo:0 320:6:56 6"
)
for pass in a b; do
  for view in 640x360 640x400; do
    extra=()
    [[ "$view" == 640x400 ]] && extra=(--deck)
    for route in "${routes[@]}"; do
      read -r name plan seq step <<<"$route"
      dir="$caps/$pass/$name-$view"
      line="$(CHECK_TIMED=1 bash scripts/room_capture.sh hazards "$plan" "$seq" "$dir" "${extra[@]}")"
      [[ "$pass" == a ]] && echo "GATE_E_ROUTE route=$name view=$view ${line#ROOM_CAPTURE }" >> "$out/capture_lines_e.txt"
      python3 scripts/room_gif.py "$dir" "$caps/$pass/route-$name-$view.gif" --step-ticks "$step"
    done
  done
done
for route in "${routes[@]}"; do
  read -r name _ <<<"$route"
  cmp "$caps/a/route-$name-640x360.gif" "$caps/b/route-$name-640x360.gif"
  cmp "$caps/a/route-$name-640x400.gif" "$caps/b/route-$name-640x400.gif"
  cp "$caps/a/route-$name-640x360.gif" "$out/route-room4-$name.gif"
  cp "$caps/a/route-$name-640x400.gif" "$out/route-room4-$name-640x400.gif"
done
echo "GATE_E_BYTE_STABLE gifs=4 routes=room4-tactical,room4-direct views=640x360,640x400 passes=2 result=identical" >> "$out/capture_lines_e.txt"
grep -q 'match=no' "$out/capture_lines_e.txt" && { echo "a window run differs from its headless run" >&2; exit 1; }
(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
