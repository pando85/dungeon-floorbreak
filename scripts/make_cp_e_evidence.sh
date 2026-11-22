#!/usr/bin/env bash
# CARL-PROG-01 CP-E evidence: real windowed captures of the pierce, one-combo shore,
# outer launch, point-blank and Juicer fixtures, at 640 x 360 and 640 x 400 (`--deck`),
# 1x and 3x, plus the 64 px reach reading (`--debug-combat`: the dotted outer-radius ring and the hurtbox overlays), assembled into
# docs/evidence/carl-prog-01/cp-e. Needs a debug binary (cargo build --locked --bin
# floor01_scene), the exported atlases (bash scripts/export_assets.sh), xvfb-run and
# Pillow. The earlier CP-B2/CP-C GIFs stay in cp-b2-c (scripts/make_cp_b2_evidence.sh).
# Every sequence is also replayed headless to its last tick and the two final
# COMBAT_STATE hashes must be equal (the presentation is read-only).
#   BIN=<path to floor01_scene> (default ./target/debug/floor01_scene)
#   CAPTURES=<scratch dir> (default artifacts/cp-e-captures)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/cp-e-captures}"
out=docs/evidence/carl-prog-01/cp-e
rm -rf "$caps"
mkdir -p "$caps" "$out"
: > "$out/capture_lines.txt"
# name|fixture|first|step|count|flags|crop (l,t,r,b of the 640 x 360 frame, empty: none)
# The crop is shown at 3x. A `--deck` capture is 640 x 400 (its crop is moved down 20 px).
seqs=(
  "pierce-goblin|carl_prog_pierce_goblin|50|2|28||250,170,450,290"
  "pierce-goblin-control|carl_prog_pierce_goblin_control|50|2|28||250,170,450,290"
  "pierce-goblin-640x400|carl_prog_pierce_goblin|50|2|28|--deck|250,190,450,310"
  "shore-one-combo|carl_prog_shore_combo|12|3|28||200,170,520,290"
  "reach-ring-outer-launch|carl_prog_dyn_outer_launch|188|3|22|--debug-combat|360,170,680,290"
  "reach-ring-wall-slam|carl_prog_dyn_wall_slam|186|2|26|--debug-combat|200,170,520,290"
  "throw-standing-640x400|carl_prog_dyn_arc_standing|4|6|36|--deck|200,190,520,310"
  "juicer-dynamite|carl_prog_juicer_dynamite_clear|100|6|40||0,60,320,290"
)
for spec in "${seqs[@]}"; do
  IFS='|' read -r name fixture first step count flags crop <<<"$spec"
  dir="$caps/$name"
  mkdir -p "$dir"
  last=$((first + step * (count - 1)))
  # shellcheck disable=SC2086
  xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 --party-fixture "$fixture" \
    $flags --capture-sequence "$first:$step:$count" --capture-dir "$dir" > "$dir/run.log" 2>&1
  # shellcheck disable=SC2086
  "$bin" --headless-floor01-smoke --party-fixture "$fixture" $flags \
    --capture-tick "$last" > "$dir/headless.log" 2>&1
  last_state() { grep '^COMBAT_STATE ' "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
  w="$(last_state "$dir/run.log")"
  h="$(last_state "$dir/headless.log")"
  match=no
  [[ -n "$w" && "$w" == "$h" ]] && match=yes
  echo "CP_E_CAPTURE name=$name fixture=$fixture first_tick=$first step=$step last_tick=$last window_hash=${w:-none} headless_hash=${h:-none} match=$match" \
    | tee -a "$out/capture_lines.txt"
  [[ "$match" == yes ]]
  python3 scripts/room_gif.py "$dir" "$out/$name-1x.gif" --step-ticks "$step"
  if [[ -n "$crop" ]]; then
    python3 scripts/room_gif.py "$dir" "$out/$name-3x.gif" --step-ticks "$step" --crop "$crop" --zoom 3
  fi
done
(cd "$out" && sha256sum ./*.gif ./*.txt | sed 's# \./# #' > SHA256SUMS)
