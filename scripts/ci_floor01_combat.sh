#!/usr/bin/env bash
# COMBAT-INT-01 fixture replays.
#   ci_floor01_combat.sh headless   no GPU: replay every fixture, compare the
#                                   COMBAT_EVENT / COMBAT_STATE lines with the
#                                   committed ones and run each twice.
#   ci_floor01_combat.sh render     inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   the real Bevy render loop (scale 3, lighting
#                                   on) must reproduce the headless events and
#                                   hash for every fixture, then capture 61
#                                   real 1920x1080 frames of cp01_single_goblin.
set -euo pipefail
mode="${1:?usage: ci_floor01_combat.sh headless|render}"
bin=./target/debug/floor01_scene
fixtures=(cp01_single_goblin cp01_two_hits cp02_two_goblins)
committed=docs/evidence/combat-int-01
out=artifacts/combat-int-01
mkdir -p "$out"

combat_lines() { grep -E '^(COMBAT_EVENT|COMBAT_STATE)' || true; }
last_state() { grep '^COMBAT_STATE' | tail -n 1 || true; }
events() { grep '^COMBAT_EVENT' || true; }
# What a render log must reproduce: every event, then the final state line (the
# log also carries intermediate COMBAT_STATE lines at load and per capture).
replay() { local text; text="$(cat)"; events <<<"$text"; last_state <<<"$text"; }

case "$mode" in
  headless)
    for f in "${fixtures[@]}"; do
      "$bin" --headless-floor01-smoke --fixture "$f" | combat_lines > "$out/headless_${f}.txt"
      "$bin" --headless-floor01-smoke --fixture "$f" | combat_lines > "$out/headless_${f}_again.txt"
      if ! cmp -s "$out/headless_${f}.txt" "$out/headless_${f}_again.txt"; then
        echo "fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed/headless_${f}.txt" "$out/headless_${f}.txt"; then
        echo "fixture $f differs from the committed replay (docs/evidence/combat-int-01)" >&2
        exit 1
      fi
      echo "COMBAT_HEADLESS_OK: $(last_state < "$out/headless_${f}.txt")"
    done
    ;;
  render)
    run_log_wait() {
      local pid="$1" log="$2" limit="$3"
      for _ in $(seq 1 "$limit"); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 1
      done
      if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        tail -n 20 "$log"
        echo "Render run did not finish" >&2
        exit 1
      fi
      if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE|FLOOR1_LIGHTING_WARNING' "$log"; then
        tail -n 20 "$log"
        echo "Render run reported a failure" >&2
        exit 1
      fi
    }
    for f in "${fixtures[@]}"; do
      headless="$(cat "$committed/headless_${f}.txt")"
      ticks="$(last_state <<<"$headless" | sed -n 's/.* tick=\([0-9]*\) .*/\1/p')"
      log="$out/render_live_${f}.log"
      "$bin" --scale 3 --fixture "$f" --exit-after-ticks "$ticks" > "$log" 2>&1 &
      run_log_wait "$!" "$log" 120
      rendered="$(replay < "$log")"
      if [[ "$rendered" != "$headless" ]]; then
        diff <(echo "$headless") <(echo "$rendered") || true
        echo "Render run of $f differs from the headless replay" >&2
        exit 1
      fi
      echo "COMBAT_LIVE_MATCH: $(last_state <<<"$rendered")"
    done
    # Real 1920x1080 capture of the CP-01 fight: ticks 118..178 every tick
    # (the first jab starts on 122, hits on 125, the goblin is defeated on 139).
    first=118; step=1; count=61
    last=$((first + step * (count - 1)))
    frames="$out/frames_cp01_single_goblin"
    log="$out/render_sequence_cp01_single_goblin.log"
    rm -rf "$frames"
    "$bin" --scale 3 --fixture cp01_single_goblin \
      --capture-sequence "${first}:${step}:${count}" --capture-dir "$frames" > "$log" 2>&1 &
    run_log_wait "$!" "$log" 600
    grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${count}" "$log"
    found="$(find "$frames" -name 'frame_*.png' | wc -l)"
    if [[ "$found" -ne "$count" ]]; then
      echo "Expected $count frames, found $found" >&2
      exit 1
    fi
    for frame in "$frames"/frame_*.png; do
      size="$(identify -format '%wx%h' "$frame")"
      if [[ "$size" != "1920x1080" ]]; then
        echo "$frame is $size, expected 1920x1080" >&2
        exit 1
      fi
    done
    "$bin" --headless-floor01-smoke --fixture cp01_single_goblin --capture-tick "$last" \
      | combat_lines > "$out/headless_cp01_single_goblin_to_${last}.txt"
    if ! diff <(replay < "$log") "$out/headless_cp01_single_goblin_to_${last}.txt"; then
      echo "Capture sequence differs from the headless replay to tick $last" >&2
      exit 1
    fi
    echo "COMBAT_SEQUENCE_MATCH: $(last_state < "$log")"
    echo "COMBAT_RENDER_OK: ${count} frames of 1920x1080, ticks ${first}..${last}"
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 2
    ;;
esac
