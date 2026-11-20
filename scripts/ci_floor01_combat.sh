#!/usr/bin/env bash
# COMBAT-INT-01 / CP-01 fixture replays.
#   ci_floor01_combat.sh headless   no GPU: replay every fixture, compare the
#                                   COMBAT_EVENT / COMBAT_STATE lines with the
#                                   committed ones (docs/evidence/cp01-feel) and
#                                   run each twice.
#   ci_floor01_combat.sh demo       no GPU: DEMO-01. The demo hall's autopilot
#                                   probes (demo01_b1, _b2, _b3, _b4, _b5, _b6 and the whole
#                                   route demo01) replay twice, at gore presets 1
#                                   to 4, and must equal the committed logs in
#                                   docs/evidence/demo-01.
#   ci_floor01_combat.sh demo-render inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   the real Bevy loop (scale 3, lighting on) must
#                                   reproduce the committed events and hash of the
#                                   whole-route probe.
#   ci_floor01_combat.sh cine       inside xvfb-run -s '-screen 0 1920x1200x24':
#                                   CINE-01 (T09, T11, T12). The real Bevy loop
#                                   on the two qualifying fixtures (the DEMO-01
#                                   B3 probe and cp03_goblin_pileup) with
#                                   --cinematic-finishes on and off at gore
#                                   presets 1 and 4: COMBAT_EVENT, COMBAT_STATE
#                                   and CINE01_MOMENT lines must be identical
#                                   to each other and to the committed headless
#                                   replays; on, the moment must start and end
#                                   (Completed) at 640x360 and 640x400, 1x and
#                                   3x; off, no slow motion line may appear.
#   ci_floor01_combat.sh render     inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   the real Bevy render loop (scale 3, lighting
#                                   on) must reproduce the headless events and
#                                   hash for every fixture, and the automatic
#                                   camera render must match its headless replay.
#   ci_floor01_combat.sh render-sequences [SHARD COUNT]
#                                   inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   capture 61 (52 for cp01_retry) real
#                                   1920x1080 frames (one per tick) of every
#                                   fixture in the `sequences` list below
#                                   (ART-INT-02 evidence). The frames drawn
#                                   (COMBAT_ANIM_STATE) and the replay must
#                                   match the headless run. With SHARD COUNT
#                                   (CI-CORRIDOR-TIME) only the sequences whose
#                                   index modulo COUNT equals SHARD run, so
#                                   COUNT parallel CI jobs (SHARD 0..COUNT-1)
#                                   together cover every sequence exactly
#                                   once; without arguments all of them run.
set -euo pipefail
# PLAYTRACE-01: CI runs never write a gameplay recording (PT-Q12).
export FLOORBREAK_RECORD=off
mode="${1:?usage: ci_floor01_combat.sh headless|render|render-sequences [SHARD COUNT]|demo|demo-render|cine}"
shard="${2:-0}"
shard_count="${3:-1}"
if ! [[ "$shard" =~ ^[0-9]+$ && "$shard_count" =~ ^[1-9][0-9]*$ ]] || (( shard >= shard_count )); then
  echo "bad shard $shard of $shard_count" >&2
  exit 2
fi
bin=./target/debug/floor01_scene
fixtures=(cp01_single_goblin cp01_two_hits cp02_two_goblins cp01_goblin_attacks cp01_interrupt cp01_haymaker_chain cp01_retry cp01_kick cp02_kick_contact cp03_shore_kill cp03_shore_miss cp03_kick_block cp03_goblin_pileup cp03_block_edge_decap qa_blast_breakup cp05_llama_slam cp05_llama_down cp05_llama_spit cp05_llama_offscreen cp05_llama_crush)
# fixture:first tick:step:count of the real-frame capture sequences. The windows
# hold the hit and the hit-stop of each fixture (see the evidence README).
sequences=(cp01_single_goblin:118:1:61 cp01_haymaker_chain:128:1:61 cp01_interrupt:122:1:61 cp01_goblin_attacks:128:1:61 cp02_two_goblins:60:1:61 cp01_retry:398:1:52 cp01_kick:128:1:61 cp02_kick_contact:20:1:61 cp03_shore_kill:98:1:61 cp05_llama_spit:30:1:61 cp05_llama_slam:140:1:61 cp05_llama_crush:120:1:61)
# Fixtures whose per-tick animation trace (the frame drawn for Carl and every
# goblin next to the sim state it follows) is committed and diffed (ART-INT-02).
traced=(cp01_haymaker_chain cp01_goblin_attacks cp02_two_goblins cp01_kick cp02_kick_contact)
committed=docs/evidence/cp01-feel
# DEMO-01 probes: name:beat (the autopilot starts at the beat's checkpoint).
demo_probes=(demo01_b1:1 demo01_b2:2 demo01_b3:3 demo01_b4:4 demo01_b5:5 demo01_b6:6 demo01:0)
demo_committed=docs/evidence/demo-01
committed_anim=docs/evidence/art-int-02
# WORK-005/006, HUD-DEBT-01/04: the live-party fixtures (two players, solo swap,
# Donut damage) with their committed replays.
party_fixtures=(duo_independent_moves duo_separation_wall solo_swap_continuity donut_hurt_iframes donut_defeat_party solo_guard_floors_at_one solo_follower_dodge)
committed_party=docs/evidence/party-2p
# HUD-DEBT-03: the Floor 01 boss encounter fixtures (the Juicer, the boss arena)
# with their committed replays. boss01_rack_donut is pending (the right rack
# waits for Q10's higher cat jump).
boss_fixtures=(boss01_pound_punish boss01_charge_jump boss01_charge_hit boss01_hug_guard boss01_rack_carl boss01_phase_roar boss01_roar_queued boss01_2p_gate boss01_solo_ai_guard boss01_party_defeat_retry boss01_crush_kill boss01_full_clear_solo boss01_full_clear_2p)
committed_boss=docs/evidence/boss-f01
# WORK-023 / HUD-DEBT-02: Donut's Q10 swat and pounce, the Magic Missile and her
# mana (the swat that breaks a tell and the light guard, the pounce that finishes
# a goblin or whiffs, the pounce on the boss, the dry press, the 350 tick regen
# and the unlock that survives a retry) with their committed replays.
donut_fixtures=(q10_swat_interrupt q10_swat_guard q10_pounce_hit q10_pounce_miss q10_dry_press q10_mana_regen q10_unlock_persists q10_pounce_boss)
committed_donut=docs/evidence/donut-q10-mana
out=artifacts/cp01-feel
mkdir -p "$out"

combat_lines() { grep -E '^(COMBAT_EVENT|COMBAT_STATE)' || true; }
last_state() { grep '^COMBAT_STATE' | tail -n 1 || true; }
last_anim() { grep '^COMBAT_ANIM_STATE' | tail -n 1 || true; }
events() { grep '^COMBAT_EVENT' || true; }
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
    # HUD-DEBT-03: every boss fixture replays identically twice, reproduces the
    # committed events and hash, and gives the same replay at gore presets 1 and
    # 4 and with the boss HUD preview flag (presentation never reaches the
    # simulation).
    for f in "${boss_fixtures[@]}"; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/boss_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/boss_${f}_again.txt"
      if ! cmp -s "$out/boss_${f}.txt" "$out/boss_${f}_again.txt"; then
        echo "boss fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed_boss/headless_${f}.txt" "$out/boss_${f}.txt"; then
        echo "boss fixture $f differs from the committed replay ($committed_boss)" >&2
        exit 1
      fi
      for preset in 1 4; do
        "$bin" --headless-floor01-smoke --party-fixture "$f" --gore-preset "$preset" | party_lines > "$out/boss_${f}_p${preset}.txt"
        if ! cmp -s "$out/boss_${f}.txt" "$out/boss_${f}_p${preset}.txt"; then
          echo "boss fixture $f at gore preset $preset differs from the default replay" >&2
          exit 1
        fi
      done
      "$bin" --headless-floor01-smoke --party-fixture "$f" --hud-preview boss | party_lines > "$out/boss_${f}_hud.txt"
      if ! cmp -s "$out/boss_${f}.txt" "$out/boss_${f}_hud.txt"; then
        echo "boss fixture $f differs with --hud-preview boss" >&2
        exit 1
      fi
      echo "BOSS_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/boss_${f}.txt")"
    done
    # WORK-023: every Donut fixture replays identically twice, reproduces the
    # committed events and hash, and gives the same replay at gore presets 1 and
    # 4 and with the HUD preview flag (presentation never reaches the sim).
    for f in "${donut_fixtures[@]}"; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/donut_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/donut_${f}_again.txt"
      if ! cmp -s "$out/donut_${f}.txt" "$out/donut_${f}_again.txt"; then
        echo "Donut fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed_donut/headless_${f}.txt" "$out/donut_${f}.txt"; then
        echo "Donut fixture $f differs from the committed replay ($committed_donut)" >&2
        exit 1
      fi
      for preset in 1 4; do
        "$bin" --headless-floor01-smoke --party-fixture "$f" --gore-preset "$preset" | party_lines > "$out/donut_${f}_p${preset}.txt"
        if ! cmp -s "$out/donut_${f}.txt" "$out/donut_${f}_p${preset}.txt"; then
          echo "Donut fixture $f at gore preset $preset differs from the default replay" >&2
          exit 1
        fi
      done
      "$bin" --headless-floor01-smoke --party-fixture "$f" --hud-preview post-unlock | party_lines > "$out/donut_${f}_hud.txt"
      if ! cmp -s "$out/donut_${f}.txt" "$out/donut_${f}_hud.txt"; then
        echo "Donut fixture $f differs with --hud-preview post-unlock" >&2
        exit 1
      fi
      echo "DONUT_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/donut_${f}.txt")"
    done
    # COMBAT-FUN-02 CF02-00: the fixture registries of CF02 and COMPANION-AI-01
    # are enumerated by the binary itself, so later PRs add fixtures without
    # editing this script. Each name replays twice identically and reproduces its
    # committed replay. CF02 replays live in ONE shared, append-only folder
    # (docs/evidence/cf02/replays/headless_<fixture>.txt, every CF02 PR may add
    # files there); COMPANION replays in docs/evidence/companion-ai. Both
    # registries are empty after CF02-00, so the bodies do not run yet.
    committed_cf02=docs/evidence/cf02/replays
    committed_companion=docs/evidence/companion-ai
    # A failing listing must fail the script, never read as "no fixtures": the
    # binary's exit code is checked (set -e sees it, because the listing is not a
    # process substitution or part of a `|| true`), and a canary prefix that must
    # list names proves the listing works at all.
    list_names() {
      local listed
      listed="$("$bin" --list-fixtures "$1")" || { echo "--list-fixtures $1 failed" >&2; exit 1; }
      printf '%s' "$listed"
    }
    canary="$(list_names cp01_)"
    if [[ "$(wc -l <<<"$canary")" -lt 5 ]]; then
      echo "--list-fixtures cp01_ returned too few names (a broken listing must not pass as an empty registry)" >&2
      exit 1
    fi
    cf02_all="$(list_names cf02_)"
    companion_all="$(list_names companion_)"
    assist_all="$(list_names cf02_assist_)"
    cf02_names=()
    while IFS= read -r name; do
      [[ -z "$name" || "$name" == cf02_assist_* ]] || cf02_names+=("$name")
    done <<<"$cf02_all"
    companion_names=()
    while IFS= read -r name; do
      [[ -z "$name" ]] || companion_names+=("$name")
    done <<<"$companion_all"$'\n'"$assist_all"
    # Every name the listing prints must be runnable (a listed name that
    # --party-fixture rejects fails the run below under `set -e`).
    for f in "${cf02_names[@]}"; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/cf02_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/cf02_${f}_again.txt"
      if ! cmp -s "$out/cf02_${f}.txt" "$out/cf02_${f}_again.txt"; then
        echo "CF02 fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed_cf02/headless_${f}.txt" "$out/cf02_${f}.txt"; then
        echo "CF02 fixture $f differs from the committed replay ($committed_cf02)" >&2
        exit 1
      fi
      echo "CF02_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/cf02_${f}.txt")"
    done
    for f in "${companion_names[@]}"; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/companion_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/companion_${f}_again.txt"
      if ! cmp -s "$out/companion_${f}.txt" "$out/companion_${f}_again.txt"; then
        echo "COMPANION fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$committed_companion/headless_${f}.txt" "$out/companion_${f}.txt"; then
        echo "COMPANION fixture $f differs from the committed replay ($committed_companion)" >&2
        exit 1
      fi
      echo "COMPANION_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/companion_${f}.txt")"
    done
    # CF02-Q40 / COMPANION-AI-01 01A-wire: "in view" is the simulation scroll
    # camera, so the assistant's decisions and hashes cannot depend on the
    # presentation camera. The camera-parity fixture replays the plain run's
    # lines under both `--camera` modes.
    for cam in auto scroll; do
      "$bin" --headless-floor01-smoke --party-fixture cf02_assist_camera_parity --camera "$cam" | party_lines > "$out/companion_camera_${cam}.txt"
      if ! cmp -s "$out/companion_cf02_assist_camera_parity.txt" "$out/companion_camera_${cam}.txt"; then
        echo "cf02_assist_camera_parity replays differently with --camera $cam (the companion must not read the presentation camera)" >&2
        exit 1
      fi
    done
    echo "COMPANION_CAMERA_PARITY_OK: --camera auto and scroll replay the assistant identically"
    # COMBAT-FUN-02: `--cf02` (the DEMO hall with `cf02_rules` on) runs the CF02
    # rules, so since CF02-A it no longer equals `--demo` (that parity with the
    # rules off is `cf02_parity_off` and the `--demo` probes above). Each probe
    # under `--cf02` must replay identically twice and log its CF02_CONFIG line.
    for entry in "${demo_probes[@]}"; do
      IFS=: read -r name beat <<<"$entry"
      # The companion is off in these probes: they isolate the CF02 rules (the
      # autopilot run is not a live party, so no companion runs here either).
      cf02_a="$("$bin" --headless-floor01-smoke --cf02 --companion off --autopilot --demo-beat "$beat" --capture-tick 600 | party_lines)"
      cf02_b="$("$bin" --headless-floor01-smoke --cf02 --companion off --autopilot --demo-beat "$beat" --capture-tick 600 | party_lines)"
      if [[ -z "$cf02_a" || "$cf02_a" != "$cf02_b" ]]; then
        echo "--cf02 is not deterministic on $name" >&2
        exit 1
      fi
      echo "CF02_DEMO_REPLAY_OK: --cf02 replays $name identically twice"
      if [[ "$name" == demo01 ]]; then
        # The live flag must reach the simulation: on the whole route the CF02
        # rules change the replay. (This fails if `--cf02` stops calling
        # `CombatState::with_cf02`; the fixtures enable the rules by name and
        # would not notice.)
        plain="$("$bin" --headless-floor01-smoke --demo --autopilot --demo-beat "$beat" --capture-tick 600 | party_lines)"
        if [[ "$plain" == "$cf02_a" ]]; then
          echo "--cf02 replays $name exactly like --demo: the flag no longer reaches the rules" >&2
          exit 1
        fi
        echo "CF02_FLAG_LIVE_OK: --cf02 differs from --demo on $name"
      fi
    done
    # AUDIENCE-01 part C: the per-hero viewer observer and the viewer HUD
    # previews are presentation only. Each probe run (the QA unlock) and each
    # `--hud-preview viewers*` run must replay the plain run's events and
    # state lines (the legacy hashes and goldens are never touched), and the
    # AUDIENCE_STATE digests (a Donut kill credited to hero 1, the boss rack,
    # a shore block, a launched kill, a mixed-chain CINE award, the demo route) must be deterministic
    # and equal the committed ones. Floor 01 itself never unlocks the
    # audience, so the plain runs above print no AUDIENCE_STATE line.
    committed_audience=docs/evidence/audience-01
    audience_probes=(
      "cp03_shore_kill|--fixture cp03_shore_kill"
      "cp01_kick|--fixture cp01_kick"
      "cp03_goblin_pileup|--fixture cp03_goblin_pileup"
      "q10_pounce_hit|--party-fixture q10_pounce_hit"
      "solo_swap_continuity|--party-fixture solo_swap_continuity"
      "boss01_crush_kill|--party-fixture boss01_crush_kill"
      "demo01_b3|--demo --autopilot --demo-beat 3 --capture-tick 600"
    )
    : > "$out/audience_probe.txt"
    for entry in "${audience_probes[@]}"; do
      IFS='|' read -r name driver <<<"$entry"
      plain="$("$bin" --headless-floor01-smoke $driver | party_lines)"
      if grep -q AUDIENCE_STATE <("$bin" --headless-floor01-smoke $driver); then
        echo "a plain run of $name printed AUDIENCE_STATE (the audience must stay locked)" >&2
        exit 1
      fi
      probed="$("$bin" --headless-floor01-smoke $driver --audience-probe)"
      again="$("$bin" --headless-floor01-smoke $driver --audience-probe)"
      if [[ "$probed" != "$again" ]]; then
        echo "the audience probe of $name is not deterministic between two runs" >&2
        exit 1
      fi
      if [[ "$(party_lines <<<"$probed")" != "$plain" ]]; then
        echo "--audience-probe changed the replay of $name" >&2
        exit 1
      fi
      for preview in viewers viewers-rising viewers-falling viewers-max; do
        previewed="$("$bin" --headless-floor01-smoke $driver --hud-preview "$preview" | party_lines)"
        if [[ "$previewed" != "$plain" ]]; then
          echo "--hud-preview $preview changed the replay of $name" >&2
          exit 1
        fi
      done
      echo "# $name: $driver" >> "$out/audience_probe.txt"
      grep '^AUDIENCE_STATE' <<<"$probed" >> "$out/audience_probe.txt"
      echo "AUDIENCE_HEADLESS_OK: $name $(grep '^AUDIENCE_STATE' <<<"$probed")"
    done
    if ! diff "$committed_audience/headless_audience_probe.txt" "$out/audience_probe.txt"; then
      echo "the audience probe digests differ from the committed ones ($committed_audience)" >&2
      exit 1
    fi
    # DEMO-01 PR C: the presentation camera never feeds the simulation. On the
    # tall camera test room, every --camera (the scroll camera, the automatic one
    # with a synthetic foe that takes aim, and the fixed coverages) replays the
    # same scene state and combat hash for the same fixture.
    for f in cp01_haymaker_chain cp03_goblin_pileup; do
      reference=""
      for cam in scroll auto 1.0 1.25 1.5; do
        got="$("$bin" --headless-floor01-smoke --fixture "$f" --camera-test-room 1 --camera "$cam" --camera-test-threat 880:30:150 --view-trace | grep -E '^(FLOOR1_SCENE_STATE|FLOOR1_VIEW_TRACE_HASH)')"
        if [[ -z "$reference" ]]; then
          reference="$got"
        elif [[ "$got" != "$reference" ]]; then
          diff <(echo "$reference") <(echo "$got") || true
          echo "fixture $f replays differently with --camera $cam (the camera must be presentation only)" >&2
          exit 1
        fi
      done
      echo "CAMERA_HASH_OK: $f (identical for scroll, auto, 1.0, 1.25 and 1.5)"
    done
    # The same on the sample corridor (no test room): its 400 px height keeps
    # every camera at 640, and the replay is still identical.
    reference=""
    for cam in scroll auto 1.5; do
      got="$("$bin" --headless-floor01-smoke --fixture cp01_haymaker_chain --camera "$cam" --view-trace | grep -E '^(FLOOR1_SCENE_STATE|FLOOR1_VIEW_TRACE_HASH)')"
      if [[ -z "$reference" ]]; then
        reference="$got"
      elif [[ "$got" != "$reference" ]]; then
        echo "corridor replay differs with --camera $cam" >&2
        exit 1
      fi
    done
    echo "CAMERA_HASH_OK: cp01_haymaker_chain on the corridor (scroll, auto, 1.5)"
    # And on the real demo hall (DEMO-01 PR A), where `auto` is the default and
    # the autopilot's route respawns Carl: the same replay for every camera.
    reference=""
    for cam in scroll auto 1.5; do
      got="$("$bin" --headless-floor01-smoke --demo --demo-beat 4 --camera "$cam" --camera-test-threat 2560:30:150 --capture-tick 420 --view-trace | grep -E '^(FLOOR1_SCENE_STATE|FLOOR1_VIEW_TRACE_HASH)')"
      if [[ -z "$reference" ]]; then
        reference="$got"
      elif [[ "$got" != "$reference" ]]; then
        echo "demo replay differs with --camera $cam" >&2
        exit 1
      fi
    done
    echo "CAMERA_HASH_OK: demo beat 4 on the hall (scroll, auto, 1.5)"
    ;;
  render)
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
    # DEMO-01 PR C: the real render loop with the automatic camera (zoom to 960
    # on the tall test room, the A1 lighting pass retargeted every frame) ends in
    # the same scene state as the headless replay, and really widened.
    log="$out/render_camera_auto.log"
    "$bin" --scale 3 --fixture cp01_haymaker_chain --camera-test-room 1 --camera auto \
      --camera-test-threat 880:30:150 --exit-after-ticks 230 > "$log" 2>&1 &
    run_log_wait "$!" "$log" 180
    expected="$("$bin" --headless-floor01-smoke --fixture cp01_haymaker_chain --camera-test-room 1 --camera auto \
      --camera-test-threat 880:30:150 --view-trace | grep '^FLOOR1_SCENE_STATE' | tail -n 1)"
    if [[ "$(grep '^FLOOR1_SCENE_STATE' "$log" | tail -n 1)" != "$expected" ]]; then
      echo "Render run with --camera auto differs from the headless replay" >&2
      exit 1
    fi
    if ! grep -q 'FLOOR1_CAMERA_REST .* rest=960' "$log"; then
      echo "The automatic camera never widened to 960 in the render run" >&2
      exit 1
    fi
    echo "CAMERA_RENDER_OK: $expected"
    ;;
  render-sequences)
    # Real 1920x1080 captures, every tick of each window (this shard's share).
    for index in "${!sequences[@]}"; do
      (( index % shard_count == shard )) || continue
      spec="${sequences[$index]}"
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
  demo)
    for spec in "${demo_probes[@]}"; do
      IFS=: read -r f beat <<<"$spec"
      run() { "$bin" --headless-floor01-smoke --demo --demo-beat "$beat" --autopilot "$@"; }
      run | combat_lines > "$out/headless_${f}.txt"
      run | combat_lines > "$out/headless_${f}_again.txt"
      if ! cmp -s "$out/headless_${f}.txt" "$out/headless_${f}_again.txt"; then
        echo "demo probe $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "$demo_committed/headless_${f}.txt" "$out/headless_${f}.txt"; then
        echo "demo probe $f differs from the committed replay ($demo_committed)" >&2
        exit 1
      fi
      # The gore preset is presentation only: same events and hash at 1 to 4.
      for preset in 1 2 3 4; do
        run --gore-preset "$preset" | combat_lines > "$out/headless_${f}_p${preset}.txt"
        if ! diff "$demo_committed/headless_${f}.txt" "$out/headless_${f}_p${preset}.txt"; then
          echo "demo probe $f at gore preset $preset differs from the committed replay" >&2
          exit 1
        fi
      done
      echo "DEMO_HEADLESS_OK: $(last_state < "$out/headless_${f}.txt")"
    done
    ;;
  demo-render)
    log="$out/render_live_demo01.log"
    committed_log="$demo_committed/headless_demo01.txt"
    ticks="$(last_state < "$committed_log" | sed -n 's/.* elapsed=\([0-9]*\) .*/\1/p')"
    "$bin" --scale 3 --demo --autopilot --exit-after-ticks "$ticks" > "$log" 2>&1 &
    pid="$!"
    for _ in $(seq 1 240); do
      kill -0 "$pid" 2>/dev/null || break
      sleep 1
    done
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      tail -n 20 "$log"
      echo "Demo render run did not finish" >&2
      exit 1
    fi
    if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE|FLOOR1_LIGHTING_WARNING' "$log"; then
      tail -n 20 "$log"
      echo "Demo render run reported a failure" >&2
      exit 1
    fi
    if ! diff <(replay < "$log") "$committed_log"; then
      echo "The demo render run differs from the committed headless replay" >&2
      exit 1
    fi
    grep -q '^DEMO01_COMPLETE' "$log"
    echo "DEMO_LIVE_MATCH: $(last_state < "$log")"
    ;;
  cine)
    # name:committed replay:expected CINE01_MOMENT prefix:ticks:flags
    cine_probes=(
      "b3|$demo_committed/headless_demo01_b3.txt|CINE01_MOMENT tick=159 tier=ordinary score=6 credited=0 target=4 precision_edge=1|300|--demo --demo-beat 3 --autopilot"
      "pileup|$committed/headless_cp03_goblin_pileup.txt|CINE01_MOMENT tick=139 tier=ordinary score=7 credited=0 target=6 precision_edge=0 mixed_chain=1|260|--fixture cp03_goblin_pileup"
    )
    cine_run() { # log, then the flags
      local log="$1"; shift
      timeout 300 "$bin" "$@" > "$log" 2>&1
      if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE|FLOOR1_LIGHTING_WARNING' "$log"; then
        tail -n 20 "$log"; echo "cine run $log reported a failure" >&2; exit 1
      fi
    }
    cine_lines() { grep -E '^(COMBAT_EVENT|CINE01_MOMENT)' "$1" || true; grep '^COMBAT_STATE' "$1" | tail -n 1 || true; }
    for spec in "${cine_probes[@]}"; do
      IFS='|' read -r name committed_log want ticks flags <<<"$spec"
      for preset in 1 4; do
        # off first: the p1 off run is the reference every other run is
        # diffed against (CINE-T09; the #122 review, N2).
        for setting in off on; do
          log="$out/cine_${name}_p${preset}_${setting}.log"
          # shellcheck disable=SC2086
          cine_run "$log" --scale 3 $flags --gore-preset "$preset" --exit-after-ticks "$ticks" --cinematic-finishes="$setting"
          cine_lines "$log" > "$log.lines"
          if ! diff "$out/cine_${name}_p1_off.log.lines" "$log.lines" > /dev/null 2>&1 && [ -f "$out/cine_${name}_p1_off.log.lines" ]; then
            diff "$out/cine_${name}_p1_off.log.lines" "$log.lines" >&2 || true
            echo "CINE-T09: $name differs between presets/settings (preset $preset, cinematic $setting)" >&2
            exit 1
          fi
        done
      done
      # The compare the loop cannot make against itself: preset 1, cinematic
      # on versus off, authoritative lines and the final sim hash.
      if ! cmp -s "$out/cine_${name}_p1_off.log.lines" "$out/cine_${name}_p1_on.log.lines"; then
        diff "$out/cine_${name}_p1_off.log.lines" "$out/cine_${name}_p1_on.log.lines" >&2 || true
        echo "CINE-T09: $name differs between cinematic on and off at gore preset 1" >&2
        exit 1
      fi
      cine_hash_off="$(grep '^COMBAT_STATE' "$out/cine_${name}_p1_off.log.lines" | sed -n 's/.*hash=\([0-9a-f]*\).*/\1/p')"
      cine_hash_on="$(grep '^COMBAT_STATE' "$out/cine_${name}_p1_on.log.lines" | sed -n 's/.*hash=\([0-9a-f]*\).*/\1/p')"
      if [[ -z "$cine_hash_off" || "$cine_hash_off" != "$cine_hash_on" ]]; then
        echo "CINE-T09: $name sim hash differs with cinematic finishers on (off=$cine_hash_off on=$cine_hash_on)" >&2
        exit 1
      fi
      # The authoritative part equals the committed headless replay.
      if ! diff <(grep -E '^(COMBAT_EVENT)' "$out/cine_${name}_p1_off.log.lines"; grep '^COMBAT_STATE' "$out/cine_${name}_p1_off.log.lines") "$committed_log"; then
        echo "CINE-T09: $name differs from the committed headless replay ($committed_log)" >&2
        exit 1
      fi
      grep -q "^$want" "$out/cine_${name}_p1_on.log.lines" || { echo "CINE-T03/T05: $name lacks '$want'" >&2; exit 1; }
      grep -q '^CINE01_SLOWMO start' "$out/cine_${name}_p1_on.log" || { echo "CINE: $name never started the slow motion" >&2; exit 1; }
      grep -q '^CINE01_SLOWMO end=Completed' "$out/cine_${name}_p1_on.log" || { echo "CINE: $name did not return to full speed (Completed)" >&2; exit 1; }
      if grep -q '^CINE01_SLOWMO' "$out/cine_${name}_p1_off.log"; then
        echo "CINE: $name slowed down with --cinematic-finishes=off" >&2; exit 1
      fi
      echo "CINE_T09_OK: $name $(grep '^CINE01_MOMENT' "$out/cine_${name}_p1_on.log" | head -n 1)"
      grep '^CINE01_SLOWMO end' "$out/cine_${name}_p4_on.log" | head -n 1
    done
    # T12: the B3 moment at both framings and both integer scales.
    for framing in "--scale 1" "--scale 3" "--deck --scale 1" "--deck --scale 2"; do
      log="$out/cine_b3_framing_${framing// /_}.log"
      # shellcheck disable=SC2086
      cine_run "$log" $framing --demo --demo-beat 3 --autopilot --exit-after-ticks 300 --cinematic-finishes=on
      grep -q '^CINE01_SLOWMO end=Completed' "$log" || { echo "CINE-T12: $framing did not complete the moment" >&2; exit 1; }
      echo "CINE_T12_OK: $framing $(grep '^CINE01_SLOWMO end' "$log" | head -n 1)"
    done
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 2
    ;;
esac
