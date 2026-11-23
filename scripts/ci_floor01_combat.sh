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
#   ci_floor01_combat.sh replay     inside xvfb-run -s '-screen 0 1920x1080x24':
#                                   PTR-R1. Record a synthetic --cf02 autopilot
#                                   session, `playtrace verify` PASS, then the
#                                   replay viewer (--replay-session) must print
#                                   result=STRICT with ticks and post equal to
#                                   the last TickRecord, save one window capture
#                                   at tick 300, report a cross-build session as
#                                   UNSUPPORTED (strict) and CHECKED (checked)
#                                   and a tampered one as DIVERGED (exit 1). No
#                                   human recording is read or written. Also
#                                   (FIX-TOOL) a live room session (room 6, no
#                                   pilot) closed by --exit-after-ticks, SIGTERM
#                                   and SIGINT each verifies as scope=full and
#                                   `inspect` labels it input-free.
set -euo pipefail
# PLAYTRACE-01: CI runs never write a gameplay recording (PT-Q12).
export FLOORBREAK_RECORD=off
mode="${1:?usage: ci_floor01_combat.sh headless|render|render-sequences [SHARD COUNT]|demo|demo-render|cine|replay}"
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
    # ENEMY-GAMEPLAY-ENGINEER: the `eng_*` Engineer fixtures replay like the CF02
    # ones (their committed replays share the CF02 folder; `cf02_arena_eng*` are
    # already in the `cf02_` listing).
    cf02_all+=$'\n'"$(list_names eng_)"
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
    # CARL-PROG-01: the `carl_prog_*` registry, enumerated by the binary (later
    # PRs add fixtures without editing this script); replays in docs/evidence/carl-prog-01.
    for f in $(list_names carl_prog_); do { "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/carl_prog_${f}.txt"; "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines | cmp -s - "$out/carl_prog_${f}.txt" && diff docs/evidence/carl-prog-01/replays/headless_${f}.txt "$out/carl_prog_${f}.txt"; } || { echo "carl_prog fixture $f is not deterministic or differs from the committed replay" >&2; exit 1; }; echo "CARL_PROG_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/carl_prog_${f}.txt")"; done
    # DONUT-PROG-01: the `donut_prog_*` registry, enumerated by the binary (later
    # PRs add fixtures without editing this script); replays in docs/evidence/donut-prog-01.
    for f in $(list_names donut_prog_); do { "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/donut_prog_${f}.txt"; "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines | cmp -s - "$out/donut_prog_${f}.txt" && diff docs/evidence/donut-prog-01/replays/headless_${f}.txt "$out/donut_prog_${f}.txt"; } || { echo "donut_prog fixture $f is not deterministic or differs from the committed replay" >&2; exit 1; }; echo "DONUT_PROG_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/donut_prog_${f}.txt")"; done
    # CORRAL-B wire: the isolated `corral_*` llama rush and party-target fixtures,
    # enumerated by the binary (CORRAL-C and CORRAL-D append fixtures without
    # editing this script); replays in docs/evidence/corral-b-wire/replays. The
    # canary proves the listing works: an empty registry must not pass.
    corral_all="$(list_names corral_)"
    if [[ "$(wc -l <<<"$corral_all")" -lt 20 ]]; then
      echo "--list-fixtures corral_ returned too few names" >&2
      exit 1
    fi
    for f in $corral_all; do { "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/corral_${f}.txt"; "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines | cmp -s - "$out/corral_${f}.txt" && diff docs/evidence/corral-b-wire/replays/headless_${f}.txt "$out/corral_${f}.txt"; } || { echo "corral fixture $f is not deterministic or differs from the committed replay" >&2; exit 1; }; echo "CORRAL_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/corral_${f}.txt")"; done
    # DONUT-MOBILITY-01 DM-B: the reflex fixtures that run without `cf02_rules`
    # (an armed evade, the not-armed hit, the locked hit, the guild unlock);
    # replays in docs/evidence/donut-mobility-01/replays. The ram and the glob
    # fixtures need the arena kit and `cf02_rules`: they are test-only.
    # DM-C adds the two cue evidence rooms (`cf02_` / `cf02_arena_` names that the
    # `--list-fixtures cf02_` registry does not enumerate).
    for f in dm_reflex_evades_swing dm_reflex_not_armed dm_evasion_locked_before_biscuit dm_evasion_unlocks_at_guild cf02_dm_reflex_cues cf02_arena_dm_perch_jump; do
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/dm_b_${f}.txt"
      "$bin" --headless-floor01-smoke --party-fixture "$f" | party_lines > "$out/dm_b_${f}_again.txt"
      if ! cmp -s "$out/dm_b_${f}.txt" "$out/dm_b_${f}_again.txt"; then
        echo "DM-B fixture $f is not deterministic between two runs" >&2
        exit 1
      fi
      if ! diff "docs/evidence/donut-mobility-01/replays/headless_${f}.txt" "$out/dm_b_${f}.txt"; then
        echo "DM-B fixture $f differs from the committed replay (docs/evidence/donut-mobility-01/replays)" >&2
        exit 1
      fi
      echo "DM_B_HEADLESS_OK: $(grep '^PARTY_STATE' "$out/dm_b_${f}.txt")"
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
        # What this guards (CF02-REVIEW, U2): `--cf02` runs the CF02 rules and
        # `--demo` does not, and `--cf02-seed` reaches the temperament draw.
        # `--cf02` loads its own room (`cf02_arena`), so "differs from --demo" is
        # true whatever the rules do; the proof is a rules-only event: goblin
        # morale (`goblin_reaction`) runs only under `cf02_rules`. The room fixture
        # names already switch the rules on in `CombatState::new`, so this does NOT
        # fail if `--cf02` stops calling `CombatState::with_cf02` (redundant on the
        # arena path, FU-D); it fails if `--cf02` stops loading the arena or the
        # arena stops running morale. `--cf02-seed` is the one flag-only path.
        plain="$("$bin" --headless-floor01-smoke --demo --autopilot --demo-beat "$beat" --capture-tick 600 | party_lines)"
        if ! grep -q 'kind=goblin_reaction' <<<"$cf02_a"; then
          echo "--cf02 on $name logged no goblin_reaction: the arena no longer runs the CF02 rules" >&2
          exit 1
        fi
        if grep -q 'kind=goblin_reaction' <<<"$plain"; then
          echo "--demo on $name logged a goblin_reaction: the CF02 rules leaked into the demo" >&2
          exit 1
        fi
        seeded="$("$bin" --headless-floor01-smoke --cf02 --cf02-seed 7 --companion off --autopilot --demo-beat "$beat" --capture-tick 600 | party_lines)"
        if [[ "$seeded" == "$cf02_a" ]]; then
          echo "--cf02-seed 7 replays $name exactly like the default seed: the seed no longer reaches the temperament draw" >&2
          exit 1
        fi
        echo "CF02_FLAG_LIVE_OK: --cf02 runs goblin morale on $name, --demo does not, and --cf02-seed changes the draw"
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
    # BOIL-A wire: the Goblin Boiler room (`--scene f01_boiler`, which implies
    # `cf02_rules`): the scripted walk loads the room and its sidecar and
    # reaches the protected exit through the checkpoint; the smoke replays
    # identically twice and prints its marker.
    boiler_a="$("$bin" --headless-floor01-smoke --scene f01_boiler | grep -E '^(F01_BOILER_(ENTRY|SMOKE_OK)|COMBAT_STATE)')"
    boiler_b="$("$bin" --headless-floor01-smoke --scene f01_boiler | grep -E '^(F01_BOILER_(ENTRY|SMOKE_OK)|COMBAT_STATE)')"
    if [[ "$boiler_a" != "$boiler_b" ]] || ! grep -q '^F01_BOILER_SMOKE_OK' <<<"$boiler_a"; then
      echo "--scene f01_boiler did not replay identically twice with its OK marker" >&2
      exit 1
    fi
    echo "F01_BOILER_HEADLESS_OK: $(grep '^F01_BOILER_SMOKE_OK' <<<"$boiler_a")"
    # PT-SCENE: `--scene f01_late` launches each listed room under a loadout. The
    # headless smoke of each room and loadout replays identically twice and names
    # its room; the story loadout of the Boiler differs from the plain room (the
    # pin holds dynamite and Light on Your Feet), and the plain room is unchanged.
    late_lines() { "$bin" --headless-floor01-smoke --scene f01_late "$@" | grep -E '^(F01_LATE_ENTRY|F01_(BOILER|HOARDER|HAZARDS|MASTERY)_(ENTRY|SMOKE_OK)|COMBAT_STATE)'; }
    for late_args in "--f01-room boiler" "--f01-room boiler --f01-loadout all" "--f01-room juicer" "--f01-room juicer --f01-loadout all" "--f01-room hoarder" "--f01-room hoarder --f01-loadout all" "--f01-room hazards" "--f01-room hazards --f01-loadout all" "--f01-room mastery" "--f01-room mastery --f01-loadout all" "--f01-room mastery --cf02-seed 7"; do
      # shellcheck disable=SC2086
      late_a="$(late_lines $late_args)"
      # shellcheck disable=SC2086
      late_b="$(late_lines $late_args)"
      room="${late_args#--f01-room }"; room="${room%% *}"
      if [[ -z "$late_a" || "$late_a" != "$late_b" ]] || ! grep -q "^F01_LATE_ENTRY room=$room " <<<"$late_a"; then
        echo "--scene f01_late $late_args did not replay identically twice with its entry line" >&2
        exit 1
      fi
    done
    late_story="$(late_lines --f01-room boiler | grep '^COMBAT_STATE')"
    if [[ "$late_story" == "$(grep '^COMBAT_STATE' <<<"$boiler_a")" ]]; then
      echo "the f01_late story loadout did not change the Boiler's state" >&2
      exit 1
    fi
    # BOIL-E (K16): the story launcher loads the sidecar's live roster, the
    # Engineer as B3 among four enemies; the plain room stays inert.
    if ! late_lines --f01-room boiler | grep -q '^F01_BOILER_SMOKE_OK .* enemies=4 engineers=1$' \
      || ! grep -q 'roster_inert=true' <<<"$boiler_a"; then
      echo "the f01_late Boiler did not load enemies=4 engineers=1, or the plain room is not inert" >&2
      exit 1
    fi
    # HRD-A2b and HAZ-A2b: the Hoarder's room (her slot enemy loaded from the
    # sidecar roster) and the hazard room (two slot slimes, two stickers, two
    # acid patches) launch from the f01_late story loadout and replay identically
    # twice; the story loadout differs from the TEST LOADOUT (the pin holds
    # abilities), and each room's marker names its pin and roster.
    for room_marker in "hoarder F01_HOARDER_SMOKE_OK slot_enemies=1" "hazards F01_HAZARDS_SMOKE_OK slot_enemies=2"; do
      read -r slot_room slot_marker slot_count <<<"$room_marker"
      story_state="$(late_lines --f01-room "$slot_room")"
      all_state="$(late_lines --f01-room "$slot_room" --f01-loadout all)"
      if ! grep -q "^$slot_marker .* env=live $slot_count " <<<"$story_state" \
        || [[ "$(grep '^COMBAT_STATE' <<<"$story_state")" == "$(grep '^COMBAT_STATE' <<<"$all_state")" ]]; then
        echo "the f01_late $slot_room room did not load its slot roster, or its loadouts are the same run" >&2
        exit 1
      fi
    done
    # PT-MASTERY: the closing arena loads its sidecar roster (four goblins, the
    # Engineer, two llamas and, since PT-TUNE, the slime as its one slot enemy)
    # under both loadouts. Its story pin
    # holds every capability, so the story run and the TEST LOADOUT start from
    # the same kits (the loadout label is the recording's, not the state's).
    for mastery_loadout in "" "--f01-loadout all"; do
      # shellcheck disable=SC2086
      mastery_state="$(late_lines --f01-room mastery $mastery_loadout)"
      if ! grep -q '^F01_MASTERY_SMOKE_OK .* env=live slot_enemies=1 enemies=5 ' <<<"$mastery_state" \
        || ! grep -q '^F01_MASTERY_ENTRY .* goblins=4 engineers=1 llamas=2 slot=1 ' <<<"$mastery_state"; then
        echo "the f01_late mastery room did not load its sidecar roster ($mastery_loadout)" >&2
        exit 1
      fi
    done
    # The direct scenes name the same room, load their slot roster (the Hoarder's one slot enemy, the hazard room's
    # two slimes) and replay identically twice.
    for direct in f01_hoarder:F01_HOARDER:slot_enemies=1 f01_hazards:F01_HAZARDS:slot_enemies=2; do
      IFS=: read -r direct_scene direct_marker direct_slots <<<"$direct"
      direct_a="$("$bin" --headless-floor01-smoke --scene "$direct_scene" | grep -E "^(${direct_marker}_(ENTRY|SMOKE_OK)|COMBAT_STATE)")"
      direct_b="$("$bin" --headless-floor01-smoke --scene "$direct_scene" | grep -E "^(${direct_marker}_(ENTRY|SMOKE_OK)|COMBAT_STATE)")"
      if [[ -z "$direct_a" || "$direct_a" != "$direct_b" ]] || ! grep -q "^${direct_marker}_SMOKE_OK .* env=live ${direct_slots} " <<<"$direct_a"; then
        echo "--scene $direct_scene did not replay identically twice with its OK marker and ${direct_slots}" >&2
        exit 1
      fi
    done
    # EVID-ROOM (BOIL-F): every scripted room pilot (src/room_pilots.rs) clears the
    # live Boiler solo and in two-player without a defeat, replays identically twice
    # (the helper exits non-zero otherwise) and equals the committed final states.
    pilots="$(BIN="$bin" python3 scripts/room_pilot_hashes.py boiler)"
    if [[ "$pilots" != "$(cat docs/evidence/f01-boiler/pilot_hashes.txt)" ]] \
      || grep -qv ' deaths=0 complete=true ' <<<"$pilots"; then
      echo "the Boiler room pilots differ from docs/evidence/f01-boiler/pilot_hashes.txt, or one was defeated or did not finish" >&2
      echo "$pilots" >&2
      exit 1
    fi
    echo "F01_BOILER_PILOTS_OK: $(wc -l <<<"$pilots") plans, each twice identical and equal to the committed hashes"
    # PT-GATE-B: the mastery room pilot (src/room_pilots.rs, `mastery`) replays identically twice
    # (the helper exits non-zero otherwise) and equals the committed final state. Since PT-GATE-C1 the plain
    # `mastery` plan is the PT-TUNE `carl_solo` fixture at seed 0 (the live party and its assist partner), so
    # it is not defeated (deaths=0), unlike the follower-bot run it replaced.
    mastery_pilot="$(BIN="$bin" python3 scripts/room_pilot_hashes.py mastery mastery)"
    if [[ "$mastery_pilot" != "$(cat docs/evidence/f01-playtest/pilot_hashes_mastery.txt)" ]] \
      || ! grep -q ' complete=' <<<"$mastery_pilot"; then
      echo "the mastery room pilot differs from docs/evidence/f01-playtest/pilot_hashes_mastery.txt" >&2
      echo "$mastery_pilot" >&2
      exit 1
    fi
    echo "F01_MASTERY_PILOT_OK: $mastery_pilot"
    # PT-GATE-C1: the aligned room pilots (`mastery|juicer:WHO:SEED`) run the library fixture's party and seed
    # through the live party: each replays identically twice and prints its ROOM_PILOT_STATE (the fixture hash,
    # asserted equal to the library's for every (who, seed) by the unit tests). PT-GATE-E adds the hazards
    # (`hazards:WHO:SEED`) and the Boiler's live room (`boiler:WHO:direct|both:SEED`).
    for aligned in mastery:donut_by_swap:3 mastery:companion:5 juicer:carl_solo:0 juicer:companion:2 \
      hazards:carl_solo:0 hazards:donut_by_swap:4 boiler:carl_solo:direct:0 boiler:companion:both:5 \
      hoarder:carl_solo:2; do
      aligned_room="${aligned%%:*}"
      aligned_a="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room "$aligned_room" --room-pilot "$aligned" --capture-tick 1500 | grep '^ROOM_PILOT_STATE')"
      aligned_b="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room "$aligned_room" --room-pilot "$aligned" --capture-tick 1500 | grep '^ROOM_PILOT_STATE')"
      if [[ -z "$aligned_a" || "$aligned_a" != "$aligned_b" ]]; then
        echo "--room-pilot $aligned did not replay identically twice with its ROOM_PILOT_STATE" >&2
        exit 1
      fi
    done
    echo "F01_ALIGNED_PILOTS_OK: 9 aligned room pilots, each twice identical"
    # PT-GATE-D, retargeted by FA-Q21 (`hazfu05`): room 4's spectacular route baits R1, then one dynamite thrown from column
    # 648 chains R2 over the slime S2 (a Chain provoke of sticker 1, R2's blast 8 ticks after the stick's), exactly one
    # throw, no Rot hurt, the exit is reached (the scene stops before tick 1500), the final hash is the filmed run's and
    # the run replays identically twice.
    spectacular_a="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room hazards --room-pilot haz_spectacular --capture-tick 1500)"
    spectacular_b="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room hazards --room-pilot haz_spectacular --capture-tick 1500)"
    spectacular_ticks="$(sed -n 's/^F01_HAZARDS_SMOKE_OK ticks=\([0-9]*\) .*/\1/p' <<<"$spectacular_a")"
    spectacular_hash="$(grep '^COMBAT_STATE ' <<<"$spectacular_a" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+) .*/\1/')"
    spectacular_rot_hurts="$(awk '/kind=hero_hurt source=/ { split($3, a, "="); if (a[2] % 65536 == 21076) n++ } END { print n + 0 }' <<<"$spectacular_a")"
    spectacular_throws="$(grep -c 'kind=charge_thrown' <<<"$spectacular_a" || true)"
    if [[ "$spectacular_a" != "$spectacular_b" ]] \
      || ! grep -q 'kind=rot_provoked sticker=1 cause=Chain' <<<"$spectacular_a" \
      || ! grep -q 'kind=blast charge=1' <<<"$spectacular_a" \
      || [[ "$spectacular_throws" -ne 1 || "$spectacular_rot_hurts" -ne 0 ]] \
      || [[ "$spectacular_hash" != "80cfaca8118ce58e" ]] \
      || [[ -z "$spectacular_ticks" || "$spectacular_ticks" -ge 1500 ]]; then
      echo "--room-pilot haz_spectacular did not chain R2 with exactly one throw, clear the room without a Rot hurt, end on the committed final hash 80cfaca8118ce58e (the filmed run, scorecard.md) and replay identically twice (got ${spectacular_hash:-none})" >&2
      exit 1
    fi
    echo "F01_HAZ_SPECTACULAR_OK: one dynamite chains R2 over the slime (+8), no Rot hurt, room cleared, final hash 80cfaca8118ce58e (the filmed run), twice identical"
    # PT-GATE-E: room 4's tactical route (HAZ-07 pattern 2): R1 is spent by bait and retreat, R2 is provoked by proximity
    # and its blast goes off (provoke + 24, the sheet's warning), no Rot blast hurts Carl, the exit is reached (the
    # scene stops before tick 1500) and the run replays identically twice.
    tactical_a="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room hazards --room-pilot haz_tactical --capture-tick 1500)"
    tactical_b="$("$bin" --headless-floor01-smoke --scene f01_late --f01-room hazards --room-pilot haz_tactical --capture-tick 1500)"
    tactical_ticks="$(sed -n 's/^F01_HAZARDS_SMOKE_OK ticks=\([0-9]*\) .*/\1/p' <<<"$tactical_a")"
    tactical_hash="$(grep '^COMBAT_STATE ' <<<"$tactical_a" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+) .*/\1/')"
    rot_hurts="$(awk '/kind=hero_hurt source=/ { split($3, a, "="); if (a[2] % 65536 == 21076) n++ } END { print n + 0 }' <<<"$tactical_a")"
    if [[ "$tactical_a" != "$tactical_b" ]] \
      || ! grep -q 'kind=rot_provoked sticker=1 cause=Proximity' <<<"$tactical_a" \
      || ! grep -q 'kind=blast charge=1' <<<"$tactical_a" \
      || [[ "$rot_hurts" -ne 0 ]] \
      || [[ "$tactical_hash" != "8e10bd0986d23ad9" ]] \
      || [[ -z "$tactical_ticks" || "$tactical_ticks" -ge 1500 ]]; then
      echo "--room-pilot haz_tactical did not bait the second sticker, clear the room without a Rot hurt, end on the committed final hash 8e10bd0986d23ad9 (the filmed run, scorecard.md) and replay identically twice (got ${tactical_hash:-none})" >&2
      exit 1
    fi
    echo "F01_HAZ_TACTICAL_OK: second sticker baited over the slime, no Rot hurt, room cleared, final hash 8e10bd0986d23ad9 (the filmed run), twice identical"
    echo "F01_LATE_HEADLESS_OK: hoarder, boiler, hazards, juicer and mastery, story and all, each twice identical"
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
  replay)
    # PTR-R1: the replay viewer on a synthetic recording (never a human session).
    rdir="$out/replay_sessions"
    rm -rf "$rdir"; mkdir -p "$rdir"
    cargo build --locked --bin playtrace
    pt=./target/debug/playtrace
    # `--record` forces the recorder on in this scripted run (the CI default is off).
    timeout 300 "$bin" --cf02 --autopilot --record --record-dir "$rdir" --exit-after-ticks 600 --no-audio --scale 1 > "$out/replay_record.log" 2>&1
    grep -q '^PLAYTRACE_SAVED .* ticks=600 status=Completed complete=true' "$out/replay_record.log" || { tail -n 20 "$out/replay_record.log"; echo "the synthetic session was not recorded" >&2; exit 1; }
    id="$(ls "$rdir")"
    "$pt" verify "$id" --dir "$rdir" > "$out/replay_verify.log"
    grep -q '^PLAYTRACE_VERIFY .*result=PASS scope=full' "$out/replay_verify.log" || { cat "$out/replay_verify.log"; echo "the synthetic session does not verify" >&2; exit 1; }
    # FIX-TOOL: a live room (room 6, the human default path, no pilot) ends itself with --exit-after-ticks, on SIGTERM
    # and on SIGINT, and each way closes its session cleanly: `playtrace verify` PASS scope=full, and `inspect` calls
    # the input-free P1 run what it is (not a plain Human session).
    ldir="$out/replay_live_room_sessions"
    rm -rf "$ldir"; mkdir -p "$ldir"
    live_room="--no-audio --scale 1 --scene f01_late --f01-room mastery --record"
    # shellcheck disable=SC2086
    timeout 300 "$bin" $live_room --record-dir "$ldir/exit" --exit-after-ticks 120 > "$out/replay_live_exit.log" 2>&1
    grep -q '^PLAYTRACE_SAVED .* ticks=120 status=Completed complete=true' "$out/replay_live_exit.log" || { tail -n 20 "$out/replay_live_exit.log"; echo "the live room scripted exit did not close its session" >&2; exit 1; }
    # An early quit (10 ticks, about 0.2 s of wall clock) exits 0. Before the fix it crashed with 134 or 139 after the
    # session was saved: the render pipelines still compiling on the async compute pool outlived the render device at
    # teardown (a wgpu device dropped on a pool thread while the main thread tore the driver down). `--exit-after-ticks`
    # now compiles pipelines synchronously, so nothing is in flight at exit.
    for _ in 1 2 3; do
      code=0
      timeout 300 "$bin" --cf02 --autopilot --no-audio --scale 1 --exit-after-ticks 10 > "$out/replay_early_exit.log" 2>&1 || code=$?
      [[ "$code" -eq 0 ]] || { tail -n 20 "$out/replay_early_exit.log"; echo "--exit-after-ticks 10 did not exit cleanly (exit $code)" >&2; exit 1; }
    done
    # Wait (at most 30 s) for the background process $1 to exit; a window that ignores the signal fails the check
    # here (and is killed) instead of hanging the job. No `timeout` wrapper: a background job of a non-interactive
    # shell starts with SIGINT ignored, and `timeout` keeps an ignored SIGINT ignored.
    wait_gone() {
      local pid="$1"
      for _ in $(seq 1 60); do
        kill -0 "$pid" 2>/dev/null || return 0
        sleep 0.5
      done
      return 1
    }
    for sig in TERM INT; do
      sdir="$ldir/$sig"; mkdir -p "$sdir"
      # shellcheck disable=SC2086
      "$bin" $live_room --record-dir "$sdir" > "$out/replay_live_$sig.log" 2>&1 &
      app=$!
      for _ in $(seq 1 120); do
        [[ -n "$(find "$sdir" -name 'ticks-*.jsonl' -size +0 2>/dev/null | head -n 1)" ]] && break
        sleep 0.5
      done
      # A few seconds of running first: a signal in the first frames can still catch the async pipeline compiles
      # in flight (only `--exit-after-ticks` compiles them synchronously, see the early-quit check above), which
      # is not what this check is about.
      sleep 4
      kill -"$sig" "$app"
      wait_gone "$app" || { kill -KILL "$app" 2>/dev/null || true; echo "the live room ignored SIG$sig" >&2; exit 1; }
      code=0; wait "$app" || code=$?
      [[ "$code" -eq 0 ]] || { tail -n 20 "$out/replay_live_$sig.log"; echo "the live room did not exit cleanly on SIG$sig (exit $code)" >&2; exit 1; }
      grep -q '^PLAYTRACE_SAVED .* status=Completed complete=true' "$out/replay_live_$sig.log" || { tail -n 20 "$out/replay_live_$sig.log"; echo "SIG$sig did not close the live room session" >&2; exit 1; }
    done
    # The replay viewer (a window kind of its own) ends on SIGTERM and SIGINT too, and prints its result line.
    for sig in TERM INT; do
      "$bin" --no-audio --scale 1 --replay-session "$id" --record-dir "$rdir" > "$out/replay_viewer_$sig.log" 2>&1 &
      app=$!
      sleep 6
      kill -"$sig" "$app"
      wait_gone "$app" || { kill -KILL "$app" 2>/dev/null || true; echo "the replay viewer ignored SIG$sig" >&2; exit 1; }
      wait "$app" || true
      grep -q '^PLAYTRACE_REPLAY ' "$out/replay_viewer_$sig.log" || { tail -n 20 "$out/replay_viewer_$sig.log"; echo "the replay viewer printed no result after SIG$sig" >&2; exit 1; }
    done
    for run in exit TERM INT; do
      lid="$(ls "$ldir/$run")"
      "$pt" verify "$lid" --dir "$ldir/$run" --root . | grep -q '^PLAYTRACE_VERIFY .*result=PASS scope=full' || { echo "the live room '$run' session does not verify as scope=full" >&2; exit 1; }
      "$pt" inspect "$lid" --dir "$ldir/$run" | grep -q 'run_kind Human (input-free' || { echo "the live room '$run' session is not labelled input-free" >&2; exit 1; }
    done
    echo "F01_LIVE_ROOM_EXIT_OK: --exit-after-ticks, SIGTERM and SIGINT each close the room 6 session (scope=full, input-free); the replay viewer ends on SIGTERM and SIGINT"
    last_post="$(python3 -I scripts/ptr_session_tool.py last-post "$rdir/$id")"
    replay_run() { # log, then the flags; returns the viewer's exit code
      local log="$1"; shift
      local code=0
      timeout 300 "$bin" --replay-exit-at-end --no-audio --scale 2 "$@" > "$log" 2>&1 || code=$?
      if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_CAPTURE_FAILURE|FLOOR1_LIGHTING_WARNING' "$log"; then
        tail -n 20 "$log"; echo "replay run $log reported a failure" >&2; exit 1
      fi
      return "$code"
    }
    result_line() { grep '^PLAYTRACE_REPLAY ' "$1" || true; }
    # 1. Strict replay of the verified session, plus one window capture at tick 300.
    rm -f "$out/replay_tick300.png"
    replay_run "$out/replay_strict.log" --replay-session "$id" --record-dir "$rdir" \
      --replay-capture-tick 300 --replay-capture-file "$out/replay_tick300.png"
    [[ "$(result_line "$out/replay_strict.log")" == "PLAYTRACE_REPLAY session=$id result=STRICT ticks=600 post=$last_post" ]] \
      || { tail -n 20 "$out/replay_strict.log"; echo "the replay did not report STRICT with the recorded ticks and post hash" >&2; exit 1; }
    if grep -q '^PLAYTRACE_VERIFY' "$out/replay_strict.log"; then echo "the viewer printed a PLAYTRACE_VERIFY line" >&2; exit 1; fi
    grep -q '^PLAYTRACE_REPLAY_CAPTURE tick=300 ' "$out/replay_strict.log" || { echo "no window capture at tick 300" >&2; exit 1; }
    python3 -I scripts/ptr_session_tool.py is-png "$out/replay_tick300.png"
    echo "REPLAY_STRICT_OK: $(result_line "$out/replay_strict.log")"
    # Checked mode on a same-source session is still STRICT (never a downgrade).
    replay_run "$out/replay_checked_same.log" --replay-session "$id" --record-dir "$rdir" --replay-compatibility checked
    [[ "$(result_line "$out/replay_checked_same.log")" == "PLAYTRACE_REPLAY session=$id result=STRICT ticks=600 post=$last_post" ]] \
      || { echo "checked mode on the same source must report STRICT" >&2; exit 1; }
    # 2. A recording by another build: strict is UNSUPPORTED (no window, exit 1);
    # checked plays it and says CHECKED.
    xdir="$out/replay_sessions_cross"
    rm -rf "$xdir"; mkdir -p "$xdir"; cp -r "$rdir/$id" "$xdir/$id"
    python3 -I scripts/ptr_session_tool.py cross-build "$xdir/$id"
    code=0
    replay_run "$out/replay_cross_strict.log" --replay-session "$id" --record-dir "$xdir" || code=$?
    [[ "$code" == 1 ]] || { echo "strict must exit 1 on a cross-build session" >&2; exit 1; }
    result_line "$out/replay_cross_strict.log" | grep -q "result=UNSUPPORTED .*code=build_mismatch" \
      || { tail -n 5 "$out/replay_cross_strict.log"; echo "strict must report UNSUPPORTED build_mismatch" >&2; exit 1; }
    replay_run "$out/replay_cross_checked.log" --replay-session "$id" --record-dir "$xdir" --replay-compatibility checked
    [[ "$(result_line "$out/replay_cross_checked.log")" == "PLAYTRACE_REPLAY session=$id result=CHECKED ticks=600 post=$last_post" ]] \
      || { echo "checked must replay a cross-build session as CHECKED" >&2; exit 1; }
    echo "REPLAY_CHECKED_OK: $(result_line "$out/replay_cross_checked.log")"
    # 3. A resealed tamper (only the replay can notice it): DIVERGED at its tick,
    # exit 1, and no later tick is claimed.
    ddir="$out/replay_sessions_diverged"
    rm -rf "$ddir"; mkdir -p "$ddir"; cp -r "$rdir/$id" "$ddir/$id"
    python3 -I scripts/ptr_session_tool.py tamper-post "$ddir/$id" 100
    code=0
    replay_run "$out/replay_diverged.log" --replay-session "$id" --record-dir "$ddir" || code=$?
    [[ "$code" == 1 ]] || { echo "a diverged replay must exit 1" >&2; exit 1; }
    result_line "$out/replay_diverged.log" | grep -qE "^PLAYTRACE_REPLAY session=$id result=DIVERGED ticks=100 post=[0-9a-f]{16} code=post_state tick=100$" \
      || { tail -n 8 "$out/replay_diverged.log"; echo "the tampered session must report DIVERGED at tick 100" >&2; exit 1; }
    echo "REPLAY_DIVERGED_OK: $(result_line "$out/replay_diverged.log")"
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 2
    ;;
esac
