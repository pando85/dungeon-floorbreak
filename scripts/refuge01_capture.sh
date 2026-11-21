#!/usr/bin/env bash
# AREA-REFUGE-01: real Bevy captures of the guild room (`floor01_scene --scene refuge_guild`), lit, under Xvfb.
# Usage: refuge01_capture.sh OUT.png SCALE [--deck]   (window = 640*SCALE x 360*SCALE, or x 400*SCALE with --deck)
# BIN overrides the binary (default ./target/debug/floor01_scene). Run from the repository root.
set -euo pipefail
out="$1"; scale="$2"; shift 2
bin="${BIN:-./target/debug/floor01_scene}"
height=360
for a in "$@"; do [[ "$a" == "--deck" ]] && height=400; done
w=$((640 * scale)); h=$((height * scale))
export FLOORBREAK_RECORD=off
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
xvfb-run -a -s "-screen 0 ${w}x${h}x24" bash -c '
  set -euo pipefail
  "$0" --scene refuge_guild --scale "$1" "${@:4}" > "$2" 2>&1 &
  pid=$!
  for _ in $(seq 1 60); do
    grep -qE "FLOOR1_TEXTURES_READY|FLOOR1_ASSET_FAILURE" "$2" && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
  done
  grep -q FLOOR1_TEXTURES_READY "$2" || { cat "$2"; kill "$pid" 2>/dev/null || true; exit 1; }
  sleep 3
  import -window root "$3"
  kill "$pid" 2>/dev/null || true
' "$bin" "$scale" "$log" "$out" "$@"
actual="$(identify -format '%wx%h' "$out")"
echo "REFUGE_CAPTURE_OK: $out $actual"
