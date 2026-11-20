#!/usr/bin/env bash
# CF02-00 gap report captures: real 640x360 frames (`--scale 1`, lighting on, gore
# preset 4) of every audited DEMO-01 spectacle, taken in the real Bevy render loop
# while the deterministic autopilot plays the beat from its checkpoint.
#
#   xvfb-run -a -s '-screen 0 1280x720x24' bash scripts/cf02_baseline_captures.sh [out_dir]
#
# Needs a debug build (cargo build --locked --bin floor01_scene) and the exported
# atlases (bash scripts/export_assets.sh). Run from the repository root. The images
# show the state at a fixed tick; they are evidence of what was drawn, not a
# game-feel verdict, and they are not reproduced bit for bit by CI (driver
# dependent); the ticks come from the headless event log (scripts/cf02_baseline_audit.py).
set -euo pipefail
bin=./target/debug/floor01_scene
out="${1:-docs/evidence/cf02/captures}"
mkdir -p "$out"
# name:driver:tick. A driver is a DEMO-01 beat number (the autopilot plays it from
# the beat's checkpoint) or a combat fixture name (its scripted input).
shots=(
  b2_contact_t75:2:75
  b2_contact_t85:2:85
  b2_contact_t100:2:100
  b2_lethal_kick_t125:2:125
  b3_block_landed_t159:3:159
  b3_block_landed_t165:3:165
  b4_llama_spit_t56:4:56
  b4_llama_slam_kick_t277:4:277
  b4_llama_slam_kick_t290:4:290
  b5_blast_t82:5:82
  b5_blast_t86:5:86
  llama_plain_down_t63:cp05_llama_down:63
  llama_plain_down_t70:cp05_llama_down:70
  llama_plain_down_t90:cp05_llama_down:90
)
for spec in "${shots[@]}"; do
  IFS=: read -r name beat tick <<<"$spec"
  if [[ "$beat" =~ ^[0-9]+$ ]]; then
    driver=(--demo --demo-beat "$beat" --autopilot)
  else
    driver=(--fixture "$beat")
  fi
  # Retry up to three times and fail (never keep a dark frame) if none is lit.
  for attempt in 1 2 3; do
    dir="$(mktemp -d)"
    "$bin" --scale 1 "${driver[@]}" \
      --capture-sequence "$((tick - 1)):1:2" --capture-dir "$dir" > "$dir/log.txt" 2>&1
    grep -q 'FLOOR1_CAPTURE_SEQUENCE_DONE count=2' "$dir/log.txt"
    # The first frame of a run is often unlit (warm-up): capture two and keep
    # the second, which is the requested tick.
    frame="$(find "$dir" -name 'frame_0001_*.png')"
    mean="$(magick "$frame" -colorspace Gray -format '%[fx:mean]' info:)"
    if awk -v m="$mean" 'BEGIN { exit !(m > 0.05) }'; then
      break
    fi
    echo "CF02_CAPTURE_RETRY: $name attempt $attempt was dark (mean $mean)" >&2
    rm -rf "$dir"
    [[ "$attempt" -lt 3 ]] || { echo "$name stayed dark" >&2; exit 1; }
  done
  grep '^COMBAT_STATE' "$dir/log.txt" | tail -n 1 > "$out/state_${name}.txt"
  size="$(identify -format '%wx%h' "$frame")"
  [[ "$size" == "640x360" ]] || { echo "$name is $size" >&2; exit 1; }
  mv "$frame" "$out/${name}_640x360.png"
  rm -rf "$dir"
  echo "CF02_CAPTURE_OK: $name beat=$beat tick=$tick"
done
(cd "$out" && sha256sum ./*.png ./state_*.txt > SHA256SUMS)
