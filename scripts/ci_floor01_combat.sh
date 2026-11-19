#!/usr/bin/env bash
# COMBAT-INT-01 / CP-01 fixture replays.
#   ci_floor01_combat.sh headless   no GPU: replay every fixture, compare the
#                                   COMBAT_EVENT / COMBAT_STATE lines with the
#                                   committed ones (docs/evidence/cp01-feel) and
#                                   run each twice.
#   ci_floor01_combat.sh render     inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   the real Bevy render loop (scale 3, lighting
#                                   on) must reproduce the headless events and
#                                   hash for every fixture, then capture 61
#                                   real 1920x1080 frames (one per tick) of
#                                   cp01_single_goblin, cp01_haymaker_chain,
#                                   cp01_interrupt, cp01_goblin_attacks and
#                                   cp02_two_goblins (ART-INT-02 evidence).
#                                   The frames drawn (COMBAT_ANIM_STATE) must
#                                   match the headless replay as well.
set -euo pipefail
mode="${1:?usage: ci_floor01_combat.sh headless|render}"
bin=./target/debug/floor01_scene
fixtures=(cp01_single_goblin cp01_two_hits cp02_two_goblins cp01_goblin_attacks cp01_interrupt cp01_haymaker_chain cp01_retry cp01_kick cp02_kick_contact cp03_shore_kill cp03_shore_miss cp03_kick_block cp03_goblin_pileup cp03_block_edge_decap qa_blast_breakup)
# fixture:first tick:step:count of the real-frame capture sequences. The windows
# hold the hit and the hit-stop of each fixture (see the evidence README).
sequences=(cp01_single_goblin:118:1:61 cp01_haymaker_chain:128:1:61 cp01_interrupt:122:1:61 cp01_goblin_attacks:128:1:61 cp02_two_goblins:60:1:61 cp01_retry:398:1:52 cp01_kick:128:1:61 cp02_kick_contact:20:1:61 cp03_shore_kill:98:1:61)
# Fixtures whose per-tick animation trace (the frame drawn for Carl and every
# goblin next to the sim state it follows) is committed and diffed (ART-INT-02).
traced=(cp01_haymaker_chain cp01_goblin_attacks cp02_two_goblins cp01_kick cp02_kick_contact)
committed=docs/evidence/cp01-feel
committed_anim=docs/evidence/art-int-02
# WORK-005/006, HUD-DEBT-01/04: the live-party fixtures (two players, solo swap,
# Donut damage) with their committed replays.
party_fixtures=(duo_independent_moves duo_separation_wall solo_swap_continuity donut_hurt_iframes donut_defeat_party solo_guard_floors_at_one solo_follower_dodge)
committed_party=docs/evidence/party-2p
out=artifacts/cp01-feel
mkdir -p "$out"

combat_lines() { grep -E '^(COMBAT_EVENT|COMBAT_STATE)' || true; }
last_state() { grep '^COMBAT_STATE' | tail -n 1 || true; }
last_anim() { grep '^COMBAT_ANIM_STATE' | tail -n 1 || true; }
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
        echo "fixture $f differs from the committed replay ($committed)" >&2
        exit 1
      fi
      echo "COMBAT_HEADLESS_OK: $(last_state < "$out/headless_${f}.txt")"
    done
    # DEATH-C: the gore preset is presentation only. Every fixture at preset 1
    # and at preset 4 (the default) replays the committed events and hash, and
    # the cosmetic presenter's own summary is deterministic between two runs.
    death_line() { grep '^DEATH_PRESENT' || true; }
    for f in "${fixtures[@]}"; do
      for preset in 1 2 3 4; do
        "$bin" --headless-floor01-smoke --fixture "$f" --gore-preset "$preset" --death-trace > "$out/gore_${f}_p${preset}.txt"
        combat_lines < "$out/gore_${f}_p${preset}.txt" > "$out/gore_${f}_p${preset}_combat.txt"
        if ! diff "$committed/headless_${f}.txt" "$out/gore_${f}_p${preset}_combat.txt"; then
          echo "fixture $f at gore preset $preset differs from the committed replay" >&2
          exit 1
        fi
      done
      "$bin" --headless-floor01-smoke --fixture "$f" --gore-preset 4 --death-trace | death_line > "$out/gore_${f}_p4_again.txt"
      if ! cmp -s <(death_line < "$out/gore_${f}_p4.txt") "$out/gore_${f}_p4_again.txt"; then
        echo "the death presenter of $f is not deterministic between two runs" >&2
        exit 1
      fi
      if grep -q 'kind=enemy_defeated' "$committed/headless_${f}.txt" \
        && cmp -s <(death_line < "$out/gore_${f}_p1.txt") <(death_line < "$out/gore_${f}_p4.txt"); then
        echo "preset 1 and 4 drew the same cosmetics for $f (the preset has no effect)" >&2
        exit 1
      fi
      echo "GORE_PRESET_OK: $f (hash identical at presets 1 to 4)"
    done
    # The frames drawn are deterministic and reproduce the committed trace.
    for f in "${traced[@]}"; do
      "$bin" --headless-floor01-smoke --fixture "$f" --anim-trace | grep '^COMBAT_ANIM_TRACE' > "$out/anim_trace_${f}.txt"
      if ! diff "$committed_anim/anim_trace_${f}.txt" "$out/anim_trace_${f}.txt"; then
        echo "anim trace of $f differs from the committed one ($committed_anim)" >&2
        exit 1
      fi
      echo "COMBAT_ANIM_TRACE_OK: $f $(wc -l < "$out/anim_trace_${f}.txt") ticks"
    done
    # The live party replays identically twice and reproduces the committed
    # events and hashes (the rust goldens in src/party_scene.rs pin the same
    # PARTY_STATE hashes).
    party_lines() { grep -E '^(COMBAT_EVENT|COMBAT_STATE|PARTY_STATE)' || true; }
    for f in "${party_fixtures[@]}"; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/party_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/party_${f}_again.txt"
      if ! cmp -s "$out/party_${f}.txt" "$out/party_${f}_again.txt"; then
        echo "party fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed_party/headless_${f}.txt" "$out/party_${f}.txt"; then
        echo "party fixture $f differs from the committed replay ($committed_party)" >&2
        exit 1
      fi
      echo "PARTY_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/party_${f}.txt")"
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
      # The live loop stops after this many ticks over every attempt (a restart
      # sets the attempt-local tick back to 0; `elapsed` does not).
      ticks="$(last_state <<<"$headless" | sed -n 's/.* elapsed=\([0-9]*\) .*/\1/p')"
      log="$out/render_live_${f}.log"
      "$bin" --scale 3 --fixture "$f" --exit-after-ticks "$ticks" > "$log" 2>&1 &
      run_log_wait "$!" "$log" 180
      rendered="$(replay < "$log")"
      if [[ "$rendered" != "$headless" ]]; then
        diff <(echo "$headless") <(echo "$rendered") || true
        echo "Render run of $f differs from the headless replay" >&2
        exit 1
      fi
      # The frames drawn at the end of the run equal the headless ones.
      expected_anim="$("$bin" --headless-floor01-smoke --fixture "$f" | last_anim)"
      if [[ "$(last_anim < "$log")" != "$expected_anim" ]]; then
        echo "Frames drawn by the render run of $f differ from the headless replay" >&2
        diff <(echo "$expected_anim") <(last_anim < "$log") || true
        exit 1
      fi
      echo "COMBAT_LIVE_MATCH: $(last_state <<<"$rendered")"
    done
    # Real 1920x1080 captures, every tick of each window.
    for spec in "${sequences[@]}"; do
      IFS=: read -r f first step count <<<"$spec"
      last=$((first + step * (count - 1)))
      frames="$out/frames_${f}"
      log="$out/render_sequence_${f}.log"
      rm -rf "$frames"
      "$bin" --scale 3 --fixture "$f" \
        --capture-sequence "${first}:${step}:${count}" --capture-dir "$frames" > "$log" 2>&1 &
      run_log_wait "$!" "$log" 900
      grep -q "FLOOR1_CAPTURE_SEQUENCE_DONE count=${count}" "$log"
      found="$(find "$frames" -name 'frame_*.png' | wc -l)"
      if [[ "$found" -ne "$count" ]]; then
        echo "Expected $count frames of $f, found $found" >&2
        exit 1
      fi
      for frame in "$frames"/frame_*.png; do
        size="$(identify -format '%wx%h' "$frame")"
        if [[ "$size" != "1920x1080" ]]; then
          echo "$frame is $size, expected 1920x1080" >&2
          exit 1
        fi
      done
      "$bin" --headless-floor01-smoke --fixture "$f" --capture-tick "$last" \
        | combat_lines > "$out/headless_${f}_to_${last}.txt"
      if ! diff <(replay < "$log") "$out/headless_${f}_to_${last}.txt"; then
        echo "Capture sequence of $f differs from the headless replay to tick $last" >&2
        exit 1
      fi
      expected_anim="$("$bin" --headless-floor01-smoke --fixture "$f" --capture-tick "$last" | last_anim)"
      if [[ "$(last_anim < "$log")" != "$expected_anim" ]]; then
        echo "Frames drawn in the capture of $f differ from the headless replay to tick $last" >&2
        exit 1
      fi
      echo "COMBAT_SEQUENCE_MATCH: $(last_state < "$log")"
      echo "COMBAT_RENDER_OK: $f ${count} frames of 1920x1080, ticks ${first}..${last}"
    done
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 2
    ;;
esac
