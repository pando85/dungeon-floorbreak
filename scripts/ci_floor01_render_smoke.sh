#!/usr/bin/env bash
# Run inside xvfb-run with the target display resolution.
# Usage: ci_floor01_render_smoke.sh desktop|deck|advanced|advanced-deck|collision
set -euo pipefail
mode="${1:-desktop}"
mkdir -p artifacts
# Scripted capture position: walk_right for 250 ticks puts the camera across the
# end of B1 and the start of C1 (see docs/03-tech/floor01-scene.md).
advanced_ticks=250
case "$mode" in
  desktop) suffix="640x360"; expected="1280x720"; args=(); frozen=0 ;;
  deck) suffix="640x400"; expected="1280x800"; args=(--deck); frozen=0 ;;
  advanced) suffix="advanced-640x360"; expected="1280x720"
    args=(--scenario walk_right --capture-tick "$advanced_ticks"); frozen=1 ;;
  advanced-deck) suffix="advanced-640x400"; expected="1280x800"
    args=(--deck --scenario walk_right --capture-tick "$advanced_ticks"); frozen=1 ;;
  collision) suffix="collision-640x360"; expected="1280x720"
    args=(--scenario walk_right --capture-tick 80 --debug-collision); frozen=1 ;;
  *) echo "Unknown capture mode: $mode" >&2; exit 2 ;;
esac
base="artifacts/floor01-scene-${suffix}"
./target/debug/floor01_scene "${args[@]}" > "${base}.log" 2>&1 &
pid=$!
cleanup() {
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
}
trap cleanup EXIT
# The binary has its own 10 s asset watchdog; allow a little more for startup.
for _ in $(seq 1 40); do
  if grep -qE 'FLOOR1_TEXTURES_READY|FLOOR1_ASSET_FAILURE' "${base}.log"; then
    break
  fi
  if ! kill -0 "$pid" 2>/dev/null; then
    break
  fi
  sleep 1
done
if grep -q FLOOR1_ASSET_FAILURE "${base}.log"; then
  cat "${base}.log"
  echo "Floor 1 scene reported an asset failure" >&2
  exit 1
fi
if ! grep -q FLOOR1_TEXTURES_READY "${base}.log"; then
  cat "${base}.log"
  echo "Floor 1 textures did not load" >&2
  exit 1
fi
if ! xwininfo -root -tree | grep -Fq 'THE CRAWL - Floor 1 Corridor'; then
  echo "Floor 1 scene window missing" >&2
  exit 1
fi
sleep 2
if ! kill -0 "$pid" 2>/dev/null; then
  cat "${base}.log"
  echo "Floor 1 scene exited after loading" >&2
  exit 1
fi
import -window root "${base}.png"
actual="$(identify -format '%wx%h' "${base}.png")"
colors="$(identify -format '%k' "${base}.png")"
if [[ "$actual" != "$expected" || "$colors" -lt 64 ]]; then
  echo "Bad screenshot: $actual, $colors colors (expected $expected, at least 64 colors)" >&2
  exit 1
fi

if [[ "$frozen" -eq 1 ]]; then
  # Deterministic capture: the render must show exactly the state the pure
  # simulation reaches headlessly.
  headless_args=("${args[@]}")
  expected_state="$(./target/debug/floor01_scene --headless-floor01-smoke "${headless_args[@]}" | grep '^FLOOR1_SCENE_STATE' | tail -n 1)"
  rendered_state="$(grep '^FLOOR1_SCENE_STATE' "${base}.log" | tail -n 1)"
  if [[ -z "$expected_state" || "$expected_state" != "$rendered_state" ]]; then
    echo "State mismatch between headless and render" >&2
    echo "  headless: $expected_state" >&2
    echo "  rendered: $rendered_state" >&2
    exit 1
  fi
  echo "FLOOR1_STATE_MATCH: $rendered_state"
fi

case "$mode" in
  advanced|advanced-deck)
    start="artifacts/floor01-scene-${suffix#advanced-}.png"
    if [[ -f "$start" ]]; then
      # The scripted capture must differ clearly from the start view.
      metric="$(compare -metric RMSE "$start" "${base}.png" null: 2>&1 || true)"
      normalized="$(sed -n 's/.*(\(.*\)).*/\1/p' <<<"$metric")"
      if [[ -z "$normalized" ]] || ! awk -v v="$normalized" 'BEGIN { exit !(v > 0.03) }'; then
        echo "Advanced capture is near-identical to the start view: RMSE '$metric'" >&2
        exit 1
      fi
      echo "FLOOR1_ADVANCED_DIFFERS: normalized RMSE $normalized vs start view"
    fi
    ;;
esac
echo "FLOOR1_RENDER_OK: $mode at $actual ($colors colors)"
