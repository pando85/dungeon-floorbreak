#!/usr/bin/env bash
# Run inside xvfb-run with the target display resolution.
# Usage: ci_floor01_render_smoke.sh desktop|desktop-720|deck|advanced|advanced-deck|nolight|nolight-advanced|reference|collision|live-lit|live-unlit|perf-lit|perf-unlit|anim-sequence|hud|hud-deck
# desktop is the primary 1920x1080 (x3) capture; desktop-720 is 1280x720 (x2).
set -euo pipefail
mode="${1:-desktop}"
mkdir -p artifacts
# Scripted capture position: walk_right for 250 ticks puts the camera across the
# end of B1 and the start of C1 (see docs/03-tech/floor01-scene.md).
advanced_ticks=250
# ANIM-INT-01 capture: the run_jump scenario, real 1920x1080 window screenshots
# every 3 fixed ticks (ticks 142, 145, ... 382). The ticks are chosen so the
# running jump's takeoff+1, apex and touchdown ticks (172, 196, 220) are captured.
anim_first=142
anim_step=3
anim_count=81
anim_last=$((anim_first + anim_step * (anim_count - 1)))
anim_dir="artifacts/anim-int-01"
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
  anim-sequence) suffix="anim-sequence-1920x1080"; expected="1920x1080"
    args=(--scale 3 --scenario run_jump --capture-sequence "${anim_first}:${anim_step}:${anim_count}"
      --capture-dir "${anim_dir}/frames"); frozen=0 ;;
  hud) suffix="hud-640x360"; expected="1280x720"; args=(); frozen=0 ;;
  hud-deck) suffix="hud-640x400"; expected="1280x800"; args=(--deck); frozen=0 ;;
  *) echo "Unknown capture mode: $mode" >&2; exit 2 ;;
esac
if [[ "$mode" == hud || "$mode" == hud-deck ]]; then
  # HUD-HYBRID-A-01 recipe: normal play, each preview fixture, two real combat
  # states (a real HeroHurt flash and a real System notice) and the option
  # flags, each a frozen capture of a deterministic state. Run inside
  # xvfb-run at the target resolution (1280x720 or 1280x800, scale 2).
  deck_flag=0
  deck_args=()
  if [[ "$mode" == hud-deck ]]; then deck_flag=1; deck_args=(--deck); fi
  hud_dir="artifacts/hud"
  mkdir -p "$hud_dir"
  # name|preview (or '-')|driver args|extra HUD args
  hud_states=(
    "normal|-|--scenario walk_right --capture-tick $advanced_ticks|"
    "pre-unlock|pre-unlock|--scenario walk_right --capture-tick $advanced_ticks|"
    "post-unlock|post-unlock|--scenario walk_right --capture-tick $advanced_ticks|"
    "depleted|depleted|--scenario walk_right --capture-tick $advanced_ticks|"
    "boss|boss|--scenario walk_right --capture-tick $advanced_ticks|"
    "coop|coop|--scenario walk_right --capture-tick $advanced_ticks|"
    "post-unlock-dualsense|post-unlock|--scenario walk_right --capture-tick $advanced_ticks|--glyphs=dualsense"
    "depleted-reduced-motion|depleted|--scenario walk_right --capture-tick $advanced_ticks|--hud-reduced-motion"
    "real-hurt|-|--fixture cp01_goblin_attacks --capture-tick 250|"
    "real-notice-launched|-|--fixture cp01_kick --capture-tick 300|"
    "real-notice-flattened|-|--fixture cp03_shore_kill --capture-tick 170|"
  )
  for entry in "${hud_states[@]}"; do
    IFS='|' read -r name preview driver extra <<<"$entry"
    hud_flag=()
    [[ "$preview" != "-" ]] && hud_flag=(--hud-preview "$preview")
    state_args=(${driver} ${extra} "${hud_flag[@]}")
    out="${hud_dir}/hud-${name}-${suffix#hud-}"
    ./target/debug/floor01_scene "${deck_args[@]}" "${state_args[@]}" > "${out}.log" 2>&1 &
    pid=$!
    for _ in $(seq 1 40); do
      grep -qE 'FLOOR1_TEXTURES_READY|FLOOR1_ASSET_FAILURE' "${out}.log" && break
      kill -0 "$pid" 2>/dev/null || break
      sleep 1
    done
    if ! grep -q FLOOR1_TEXTURES_READY "${out}.log"; then
      cat "${out}.log"; kill "$pid" 2>/dev/null || true
      echo "HUD capture $name: textures did not load" >&2; exit 1
    fi
    sleep 2
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "${out}.log"; echo "HUD capture $name: scene exited" >&2; exit 1
    fi
    import -window root "${out}.png"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    if grep -qE 'FLOOR1_LIGHTING_WARNING|FLOOR1_HUD_WARNING|FLOOR1_ASSET_FAILURE' "${out}.log"; then
      cat "${out}.log"; echo "HUD capture $name: warning in the log" >&2; exit 1
    fi
    actual="$(identify -format '%wx%h' "${out}.png")"
    if [[ "$actual" != "$expected" ]]; then
      echo "HUD capture $name is $actual, expected $expected" >&2; exit 1
    fi
    # The rendered simulation state equals the headless one, with or without
    # the fixture flag (a fixture never touches a replay).
    rendered_state="$(grep '^FLOOR1_SCENE_STATE' "${out}.log" | tail -n 1)"
    plain_headless="$(./target/debug/floor01_scene --headless-floor01-smoke "${deck_args[@]}" ${driver} | grep -v '^FLOOR1_HEADLESS_OK')"
    flag_headless="$(./target/debug/floor01_scene --headless-floor01-smoke "${deck_args[@]}" ${driver} ${extra} "${hud_flag[@]}" | grep -v '^FLOOR1_HEADLESS_OK')"
    if [[ "$plain_headless" != "$flag_headless" ]]; then
      echo "HUD flags changed the headless replay of $name" >&2; exit 1
    fi
    if [[ -z "$rendered_state" ]] || ! grep -qxF "$rendered_state" <<<"$plain_headless"; then
      echo "HUD capture $name: rendered state differs from the headless replay" >&2
      echo "  rendered: $rendered_state" >&2; exit 1
    fi
    shown_preview=""; [[ "$preview" != "-" ]] && shown_preview="$preview"
    python3 -I scripts/check_hud_capture.py "${out}.png" 2 "$deck_flag" $shown_preview
    echo "HUD_STATE_OK: $name"
  done
  echo "FLOOR1_RENDER_OK: $mode (${#hud_states[@]} HUD captures at $expected in $hud_dir)"
  exit 0
fi
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
  # Animation is presentation only but must be deterministic too: the live
  # render loop and the headless run show the same frames.
  live_anim="$(grep '^FLOOR1_ANIM_STATE' "${base}.log" | tail -n 1)"
  headless_anim="$(./target/debug/floor01_scene --headless-floor01-smoke --scenario walk_right --capture-tick "$advanced_ticks" | grep '^FLOOR1_ANIM_STATE' | tail -n 1)"
  if [[ -z "$live_anim" || "$live_anim" != "$headless_anim" ]]; then
    echo "Live animation state differs from the headless run" >&2
    echo "  headless: $headless_anim" >&2
    echo "  live:     $live_anim" >&2
    exit 1
  fi
  echo "FLOOR1_LIVE_ANIM_MATCH: $live_anim"
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
if [[ "$mode" == anim-sequence ]]; then
  # The binary steps the scenario by hand and saves one window screenshot per
  # step, then exits. Check every frame, then the final state against the
  # headless simulation, and keep the headless per-tick trace for the evidence.
  for _ in $(seq 1 600); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
  done
  if kill -0 "$pid" 2>/dev/null; then
    tail -n 20 "${base}.log"
    echo "Capture sequence did not finish" >&2
    exit 1
  fi
  if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE' "${base}.log" \
    || ! grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${anim_count}" "${base}.log"; then
    tail -n 20 "${base}.log"
    echo "Capture sequence failed" >&2
    exit 1
  fi
  frames="$(find "${anim_dir}/frames" -name 'frame_*.png' | wc -l)"
  if [[ "$frames" -ne "$anim_count" ]]; then
    echo "Expected $anim_count frames, found $frames" >&2
    exit 1
  fi
  for frame in "${anim_dir}"/frames/frame_*.png; do
    size="$(identify -format '%wx%h' "$frame")"
    if [[ "$size" != "$expected" ]]; then
      echo "$frame is $size, expected $expected" >&2
      exit 1
    fi
  done
  ./target/debug/floor01_scene --headless-floor01-smoke --scenario run_jump \
    --capture-tick "$anim_last" --anim-trace > "${anim_dir}/anim_trace_full.txt"
  headless_state="$(grep '^FLOOR1_SCENE_STATE' "${anim_dir}/anim_trace_full.txt" | tail -n 1)"
  headless_anim="$(grep '^FLOOR1_ANIM_STATE' "${anim_dir}/anim_trace_full.txt" | tail -n 1)"
  rendered_state="$(grep '^FLOOR1_SCENE_STATE' "${base}.log" | tail -n 1)"
  rendered_anim="$(grep '^FLOOR1_ANIM_STATE' "${base}.log" | tail -n 1)"
  if [[ "$headless_state" != "$rendered_state" || "$headless_anim" != "$rendered_anim" ]]; then
    echo "Final state differs between the headless run and the capture sequence" >&2
    echo "  headless: $headless_state / $headless_anim" >&2
    echo "  rendered: $rendered_state / $rendered_anim" >&2
    exit 1
  fi
  echo "FLOOR1_ANIM_SEQUENCE_MATCH: $rendered_state / $rendered_anim"
  # Per-tick strips: the 3-tick sequence cannot show a one-tick event, so the
  # takeoff (impulse tick 171) and the landing (touchdown 220, run cut 225)
  # are captured again at every tick.
  for strip in "takeoff:168:1:15" "landing:218:1:13"; do
    IFS=: read -r name first step count <<<"$strip"
    strip_log="${anim_dir}/strip_${name}.log"
    timeout 300 ./target/debug/floor01_scene --scale 3 --scenario run_jump \
      --capture-sequence "${first}:${step}:${count}" \
      --capture-dir "${anim_dir}/frames_${name}" > "$strip_log" 2>&1 || {
      tail -n 20 "$strip_log"
      echo "Strip capture $name failed" >&2
      exit 1
    }
    if ! grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${count}" "$strip_log" \
      || grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE|FLOOR1_LIGHTING_WARNING' "$strip_log"; then
      tail -n 20 "$strip_log"
      echo "Strip capture $name did not complete" >&2
      exit 1
    fi
    echo "FLOOR1_ANIM_STRIP_OK: $name ($count frames from tick $first)"
  done
  echo "FLOOR1_RENDER_OK: $mode ($anim_count frames of $expected, ticks ${anim_first}..${anim_last})"
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
  # The animation state (tag and atlas frame per actor) is derived from the
  # simulation and must match between the headless run and the render too.
  expected_anim="$(./target/debug/floor01_scene --headless-floor01-smoke "${headless_args[@]}" | grep '^FLOOR1_ANIM_STATE' | tail -n 1)"
  rendered_anim="$(grep '^FLOOR1_ANIM_STATE' "${base}.log" | tail -n 1)"
  if [[ -z "$expected_anim" || "$expected_anim" != "$rendered_anim" ]]; then
    echo "Animation state mismatch between headless and render" >&2
    echo "  headless: $expected_anim" >&2
    echo "  rendered: $rendered_anim" >&2
    exit 1
  fi
  echo "FLOOR1_ANIM_MATCH: $rendered_anim"
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
