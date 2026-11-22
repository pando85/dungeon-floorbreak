#!/usr/bin/env bash
# PT-GATE-A evidence (plan section 10, items 3, 4 and 7), into docs/evidence/f01-playtest/:
#   1. frame time with gore preset 4 in the Boiler's full chain (scripts/room_perf.sh), three
#      runs, plus one NON-FINAL run of the mastery arena (PT-TUNE will change its waves);
#   2. the two grant moments: the Juicer's defeat (the gauntlet and the blink) played by the
#      PT-SCENE-B pilot, and the dynamite grant by the PLACEHOLDER-ZONE fixture (room 1, the
#      Hoarder, is not at HRD-B);
#   3. one recorded session per listed room (2 Boiler, 4 hazards, 5 Juicer) played by the
#      scripted pilots in the real window, verified with `playtrace verify` (PASS, scope full),
#      and the journal's CapabilityGranted lines counted. Room 6 (mastery) waits for PT-TUNE.
# Needs the debug binaries (cargo build --locked --bin floor01_scene --bin playtrace), the
# exported atlases (bash scripts/export_assets.sh), xvfb-run and Pillow. The README is hand-written.
# Never records a human: every session is a scripted pilot (run_kind ScriptedBot).
#   BIN=<floor01_scene>      (default ./target/debug/floor01_scene)
#   PT=<playtrace>           (default ./target/debug/playtrace)
#   CAPTURES=<scratch dir>   (default artifacts/f01-playtest-captures)
#   SESSIONS=<scratch dir>   (default artifacts/f01-playtest-sessions; never inside docs/)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
pt="${PT:-./target/debug/playtrace}"
caps="${CAPTURES:-artifacts/f01-playtest-captures}"
sessions="${SESSIONS:-artifacts/f01-playtest-sessions}"
out=docs/evidence/f01-playtest
rm -rf "$caps" "$sessions"
mkdir -p "$caps" "$sessions" "$out"
rm -f "$out"/*.gif "$out"/*.png "$out"/*.txt "$out"/SHA256SUMS
xvfb() { xvfb-run -a -s '-screen 0 1280x720x24' "$@"; }
last_state() { grep '^COMBAT_STATE ' "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }

# 1. Frame time (three Boiler runs: the interval is wall-clock, so it varies run to run).
: > "$out/perf.txt"
for run in 1 2 3; do
  echo "# boiler chain, run $run" >> "$out/perf.txt"
  bash scripts/room_perf.sh boiler chain 950 | grep -v '^log=' >> "$out/perf.txt"
done
echo "# mastery arena, NON-FINAL (PT-TUNE changes the waves; re-run after it)" >> "$out/perf.txt"
bash scripts/room_perf.sh mastery mastery 3000 | grep -v '^log=' >> "$out/perf.txt"

# 2. The grant moments. The Juicer: 76 frames from tick 3150, 6 ticks (100 ms) a frame; the defeat and
# both grants are tick 3175, the pilot reaches the exit on tick 3255 and the room's sim stops there, so
# the frames after that are presentation (HUD) ticks only: the notice queue shows KNOCKOUT!, then the
# grant notices HEAVY FIST (the gauntlet) and PUDDLE JUMPER (the blink), 136 ticks each.
: > "$out/grant_lines.txt"
bash scripts/room_capture.sh juicer juicer 3150:6:76 "$caps/grant-juicer" \
  | sed 's/^/GATE_A_GRANT juicer /' >> "$out/grant_lines.txt"
python3 scripts/room_gif.py "$caps/grant-juicer" "$out/grant-juicer-gauntlet-blink.gif" --step-ticks 6 \
  --still 24 "$out/still-juicer-notice-knockout-frame24.png" \
  --still 40 "$out/still-juicer-notice-heavy-fist-frame40.png" \
  --still 65 "$out/still-juicer-notice-puddle-jumper-frame65.png"
"$BIN" --headless-floor01-smoke --scene f01_late --f01-room juicer --room-pilot juicer \
  --capture-tick 3255 2>&1 | grep 'kind=capability_granted' | sed 's/^/GATE_A_JUICER_EVENT /' >> "$out/grant_lines.txt"
# The dynamite: the placeholder-zone fixture (NOT the Hoarder's room), ticks 36 to 105 (the grant is tick 69).
fx=carl_prog_story_event_grants_dynamite
mkdir -p "$caps/grant-dynamite"
xvfb "$BIN" --scale 1 --party-fixture "$fx" --capture-sequence 36:3:24 --capture-dir "$caps/grant-dynamite" \
  > "$caps/grant-dynamite/run.log" 2>&1
"$BIN" --headless-floor01-smoke --party-fixture "$fx" --capture-tick 105 > "$caps/grant-dynamite/headless.log" 2>&1
wh="$(last_state "$caps/grant-dynamite/run.log")"
hh="$(last_state "$caps/grant-dynamite/headless.log")"
match=no; [[ -n "$wh" && "$wh" == "$hh" ]] && match=yes
echo "GATE_A_GRANT dynamite (placeholder-zone fixture) fixture=$fx first_tick=36 step=3 last_tick=105 window_hash=${wh:-none} headless_hash=${hh:-none} match=$match" >> "$out/grant_lines.txt"
[[ "$match" == yes ]]
grep 'kind=capability_granted' "$caps/grant-dynamite/headless.log" | sed 's/^/GATE_A_DYNAMITE_EVENT /' >> "$out/grant_lines.txt"
python3 scripts/room_gif.py "$caps/grant-dynamite" "$out/grant-dynamite-placeholder-zone.gif" --step-ticks 3 \
  --still 11 "$out/still-dynamite-grant-tick69.png"

# 3. One recorded session per room, scripted pilot, real window, verified with the same source.
: > "$out/playtrace_sessions.txt"
record() { # label room plan ticks
  local label="$1" room="$2" plan="$3" ticks="$4" dir="$sessions/$1" log="$sessions/$1.log"
  mkdir -p "$dir"
  xvfb "$BIN" --scale 1 --no-audio --scene f01_late --f01-room "$room" --room-pilot "$plan" \
    --record --record-dir "$dir" --exit-after-ticks "$ticks" > "$log" 2>&1
  local saved id
  saved="$(grep '^PLAYTRACE_SAVED ' "$log" | sed -E 's/^(PLAYTRACE_SAVED) session=[0-9a-f]+ (.*)/\1 \2/')"
  id="$(ls "$dir")"
  {
    echo "== room=$room plan=$plan label=$label exit_after_ticks=$ticks"
    echo "$saved"
    "$pt" verify "$id" --dir "$dir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
    "$pt" inspect "$id" --dir "$dir" | grep -E '^  (schema|reset|ticks|episodes)' | sed 's/^  /inspect: /'
    # The journal's CapabilityGranted lines (one per grant), straight from the tick files.
    grep -h -o '"k":"CapabilityGranted","d":"[^}]*}' "$dir/$id"/ticks-*.jsonl \
      | sed -E 's/"k":"CapabilityGranted","d":"CapabilityGranted \{ (.*) \}/journal: CapabilityGranted \1/; s/\\"/"/g' || true
    echo "journal_capability_granted_count=$(grep -h -o '"k":"CapabilityGranted"' "$dir/$id"/ticks-*.jsonl | wc -l)"
  } >> "$out/playtrace_sessions.txt"
}
record boiler-chain boiler chain 960
record hazards-direct hazards direct 1500
record juicer juicer juicer 3300
# Every session must have passed (a FAIL line has no `result=PASS`).
[[ "$(grep -c 'result=PASS scope=full' "$out/playtrace_sessions.txt")" == 3 ]]

# The story loadout grants nothing in the Boiler and the hazard room, and exactly one gauntlet and one
# blink grant at the Juicer's defeat (one CapabilityGranted per grant in the journal).
[[ "$(grep '^journal_capability_granted_count=' "$out/playtrace_sessions.txt" | sed 's/.*=//' | tr '\n' ' ')" == "0 0 2 " ]]
[[ "$(grep -c 'journal: CapabilityGranted tick: 3175, hero: 0, capability: "fist_reinforced"' "$out/playtrace_sessions.txt")" == 1 ]]
[[ "$(grep -c 'journal: CapabilityGranted tick: 3175, hero: 1, capability: "special_short_blink"' "$out/playtrace_sessions.txt")" == 1 ]]

(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt | sed 's# \./# #' > SHA256SUMS)
