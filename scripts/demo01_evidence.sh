#!/usr/bin/env bash
# DEMO-01 evidence: one real 1920x1080 screenshot per beat of the demo hall,
# captured in the real Bevy render loop (scale 3, lighting on) while the
# deterministic autopilot plays the beat from its checkpoint, plus the probe
# replays and SHA256SUMS.
#
#   xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/demo01_evidence.sh [out_dir]
#
# Needs a debug build (cargo build --locked --bin floor01_scene) and the
# exported atlases (bash scripts/export_assets.sh). The images show the state
# at a fixed tick; they are not a game-feel claim. Run from the repository root.
set -euo pipefail
bin=./target/debug/floor01_scene
out="${1:-docs/evidence/demo-01}"
mkdir -p "$out"
# name:beat:tick of the frame. The ticks count from the beat's checkpoint.
shots=(
  b0_wakeup:0:110
  b1_first_rematch:1:91
  b2_kick_contact:2:76
  b3_block_falling:3:140
  b3_block_landed:3:160
  b4_llama_hook:4:120
  b5_lit_charge_fuse:5:60
  b5_burst:5:86
  b6_pileup:6:110
)
for spec in "${shots[@]}"; do
  IFS=: read -r name beat tick <<<"$spec"
  dir="$(mktemp -d)"
  log="$dir.log"
  "$bin" --scale 3 --demo --demo-beat "$beat" --autopilot \
    --capture-sequence "${tick}:1:1" --capture-dir "$dir" > "$log" 2>&1
  grep -q 'FLOOR1_CAPTURE_SEQUENCE_DONE count=1' "$log"
  # The sim state of the frame (the capture line itself names a temp path).
  grep '^COMBAT_STATE' "$log" | tail -n 1 > "$out/state_${name}.txt"
  frame="$(find "$dir" -name 'frame_*.png')"
  size="$(identify -format '%wx%h' "$frame")"
  [[ "$size" == "1920x1080" ]] || { echo "$name is $size" >&2; exit 1; }
  mv "$frame" "$out/${name}_1920x1080.png"
  rm -rf "$dir" "$log"
  echo "DEMO_SHOT_OK: $name beat=$beat tick=$tick"
done
(cd "$out" && sha256sum ./*.png ./headless_*.txt ./state_*.txt > SHA256SUMS)
