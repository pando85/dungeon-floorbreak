#!/usr/bin/env bash
# Run inside xvfb-run with the target display resolution.
# Usage: ci_floor01_render_smoke.sh desktop|desktop-720|deck|advanced|advanced-deck|nolight|nolight-advanced|reference|collision|live-lit|live-unlit|perf-lit|perf-unlit
# desktop is the primary 1920x1080 (x3) capture; desktop-720 is 1280x720 (x2).
set -euo pipefail
mode="${1:-desktop}"
mkdir -p artifacts
# Scripted capture position: walk_right for 250 ticks puts the camera across the
# end of B1 and the start of C1 (see docs/03-tech/floor01-scene.md).
advanced_ticks=250
case "$mode" in
  desktop) suffix="1920x1080"; expected="1920x1080"; args=(--scale 3); frozen=0 ;;
  desktop-720) suffix="640x360"; expected="1280x720"; args=(); frozen=0 ;;
  deck) suffix="640x400"; expected="1280x800"; args=(--deck); frozen=0 ;;
  advanced) suffix="advanced-640x360"; expected="1280x720"
    args=(--scenario walk_right --capture-tick "$advanced_ticks"); frozen=1 ;;
  advanced-deck) suffix="advanced-640x400"; expected="1280x800"
    args=(--deck --scenario walk_right --capture-tick "$advanced_ticks"); frozen=1 ;;
  nolight) suffix="nolight-1920x1080"; expected="1920x1080"
    args=(--no-lighting --scale 3); frozen=0 ;;
  nolight-advanced) suffix="nolight-advanced-640x360"; expected="1280x720"
    args=(--no-lighting --scenario walk_right --capture-tick "$advanced_ticks"); frozen=1 ;;
  reference) suffix="reference-1920x1080"; expected="1920x1080"
    args=(--reference-frame --scale 3); frozen=1 ;;
  live-lit) suffix="live-lit-640x360"; expected="1280x720"
    args=(--scenario walk_right --exit-after-ticks "$advanced_ticks"); frozen=0 ;;
  live-unlit) suffix="live-unlit-640x360"; expected="1280x720"
    args=(--no-lighting --scenario walk_right --exit-after-ticks "$advanced_ticks"); frozen=0 ;;
  perf-lit) suffix="perf-lit-640x360"; expected="1280x720"; args=(--perf-frames 100); frozen=0 ;;
  perf-unlit) suffix="perf-unlit-640x360"; expected="1280x720"
    args=(--no-lighting --perf-frames 100); frozen=0 ;;
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
  # The lighting budgets (lights, shadows) are proven at load; a runtime
  # warning means a light was dropped, which must fail the run.
  if grep -q FLOOR1_LIGHTING_WARNING "${base}.log" 2>/dev/null; then
    echo "FLOOR1_LIGHTING_WARNING present in ${base}.log" >&2
    exit 1
  fi
}
trap cleanup EXIT
if [[ "$mode" == live-* ]]; then
  # Live-stepped run: the real render loop steps the scripted scenario for N
  # fixed ticks (lighting systems active, or --no-lighting), prints the final
  # state and exits. Its state must equal the pure headless simulation and,
  # for the unlit run, the lit run.
  for _ in $(seq 1 120); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
  done
  if grep -q FLOOR1_ASSET_FAILURE "${base}.log"; then
    cat "${base}.log"
    echo "Floor 1 scene reported an asset failure" >&2
    exit 1
  fi
  if kill -0 "$pid" 2>/dev/null; then
    cat "${base}.log"
    echo "Live run did not finish" >&2
    exit 1
  fi
  live_state="$(grep '^FLOOR1_SCENE_STATE' "${base}.log" | tail -n 1)"
  headless_state="$(./target/debug/floor01_scene --headless-floor01-smoke --scenario walk_right --capture-tick "$advanced_ticks" | grep '^FLOOR1_SCENE_STATE' | tail -n 1)"
  if [[ -z "$live_state" || "$live_state" != "$headless_state" ]]; then
    echo "Live state differs from the headless simulation" >&2
    echo "  headless: $headless_state" >&2
    echo "  live:     $live_state" >&2
    exit 1
  fi
  echo "FLOOR1_LIVE_STATE_MATCH: $live_state"
  if [[ "$mode" == "live-unlit" && -f artifacts/floor01-scene-live-lit-640x360.log ]]; then
    lit_state="$(grep '^FLOOR1_SCENE_STATE' artifacts/floor01-scene-live-lit-640x360.log | tail -n 1)"
    if [[ "$lit_state" != "$live_state" ]]; then
      echo "Lighting changed the live simulation state" >&2
      echo "  lit:   $lit_state" >&2
      echo "  unlit: $live_state" >&2
      exit 1
    fi
    echo "FLOOR1_LIVE_LIGHTING_STATE_IDENTICAL: $live_state"
  fi
  echo "FLOOR1_RENDER_OK: $mode (live state run; no window geometry check)"
  exit 0
fi
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
window_title='THE CRAWL - Floor 1 Corridor'
if ! window_info="$(xwininfo -name "$window_title" 2>/dev/null)"; then
  echo "Floor 1 scene window missing" >&2
  exit 1
fi
# The window itself (not the Xvfb root) must be the expected integer multiple
# of the virtual resolution: 1280x720 for 640x360, 1280x800 for 640x400.
window_geometry="$(awk '/^  Width:/ {w=$2} /^  Height:/ {h=$2} END {print w "x" h}' <<<"$window_info")"
if [[ "$window_geometry" != "$expected" ]]; then
  echo "Window is $window_geometry, expected $expected" >&2
  exit 1
fi
if [[ "$mode" == perf-* ]]; then
  # Frame-interval probe: the process measures itself and exits. Software
  # Vulkan numbers are a CI health signal, not a performance claim.
  for _ in $(seq 1 300); do
    grep -q FLOOR1_PERF "${base}.log" && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
  done
  if ! grep -q FLOOR1_PERF "${base}.log"; then
    cat "${base}.log"
    echo "No FLOOR1_PERF line" >&2
    exit 1
  fi
  grep -E 'FLOOR1_PERF|AdapterInfo' "${base}.log" | head -n 3
  echo "FLOOR1_RENDER_OK: $mode window $window_geometry"
  exit 0
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
  nolight-advanced)
    lit_log="artifacts/floor01-scene-advanced-640x360.log"
    if [[ -f "$lit_log" ]]; then
      # Lighting is presentation only: the simulation state must not depend on it.
      lit_state="$(grep '^FLOOR1_SCENE_STATE' "$lit_log" | tail -n 1)"
      if [[ "$lit_state" != "$rendered_state" ]]; then
        echo "Lighting changed the simulation state" >&2
        echo "  lit:   $lit_state" >&2
        echo "  unlit: $rendered_state" >&2
        exit 1
      fi
      echo "FLOOR1_LIGHTING_STATE_IDENTICAL: $rendered_state"
    fi
    ;;
  nolight)
    lit="artifacts/floor01-scene-1920x1080.png"
    if [[ -f "$lit" ]]; then
      metric="$(compare -metric RMSE "$lit" "${base}.png" null: 2>&1 || true)"
      normalized="$(sed -n 's/.*(\(.*\)).*/\1/p' <<<"$metric")"
      if [[ -z "$normalized" ]] || ! awk -v v="$normalized" 'BEGIN { exit !(v > 0.02) }'; then
        echo "Lit and unlit captures are near-identical: RMSE '$metric'" >&2
        exit 1
      fi
      echo "FLOOR1_LIGHTING_DIFFERS: normalized RMSE $normalized vs the lit capture"
    fi
    ;;
esac
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
echo "FLOOR1_RENDER_OK: $mode window $window_geometry, screenshot $actual ($colors colors)"
