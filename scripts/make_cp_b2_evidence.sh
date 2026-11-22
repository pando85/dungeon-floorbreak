#!/usr/bin/env bash
# CARL-PROG-01 CP-B2 + CP-C evidence: real windowed captures of the dynamite
# fixtures, assembled into GIFs (1x and 3x) and HUD stills under
# docs/evidence/carl-prog-01/cp-b2-c. Needs a debug binary (cargo build --locked
# --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run and Pillow. The README is hand-written.
#   BIN=<path to floor01_scene> (default ./target/debug/floor01_scene)
#   CAPTURES=<scratch dir below ./artifacts> (default artifacts/cp-b2-captures; deleted first, and refused unless it
#     is a directory this script made before or does not exist)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/cp-b2-captures}"
# The scratch directory is deleted first: refuse anything that is not a dedicated capture directory below
# ./artifacts (CP-B2-FU-EVID-RM; the guard is scripts/evidence_captures_guard.sh).
# shellcheck source=scripts/evidence_captures_guard.sh
source scripts/evidence_captures_guard.sh
prepare_captures_dir "$caps"
# fixture : first tick : step : count : extra flags
sequences=(
  "carl_prog_dyn_arc_standing:4:6:36:--debug-combat"
  "carl_prog_dyn_arc_moving:4:5:28:"
  "carl_prog_dyn_chain_depth_cap:188:3:20:"
  "carl_prog_dyn_breaks_shore:188:4:14:"
  "carl_prog_dyn_wall_slam:188:2:12:"
  "carl_prog_dyn_bowling:188:2:14:"
  "carl_prog_dyn_rack_strut:188:4:14:"
  "carl_prog_special_locked:20:1:1:"
  "carl_prog_dyn_arc_standing:3:1:1:"
  "carl_prog_dyn_arc_standing:100:1:1:"
  "carl_prog_dyn_arc_standing:230:1:1:"
  "carl_prog_story_event_grants_dynamite:70:1:1:"
)
i=0
for spec in "${sequences[@]}"; do
  IFS=: read -r fixture first step count flags <<<"$spec"
  dir="$caps/$(printf '%02d' "$i")_${fixture}_${first}"
  mkdir -p "$dir"
  # shellcheck disable=SC2086
  xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 --party-fixture "$fixture" \
    $flags --capture-sequence "$first:$step:$count" --capture-dir "$dir" > "$dir/run.log" 2>&1
  i=$((i + 1))
done
python3 scripts/make_cp_b2_evidence.py "$caps"
