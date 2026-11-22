#!/usr/bin/env bash
# PT-GATE-B evidence (plan section 10, items 3, 4, 7 and 8), into docs/evidence/f01-playtest/:
#   1. the mastery room pilot's committed final state (pilot_hashes_mastery.txt, replayed twice);
#   2. the mastery wave GIF (W2, real Bevy, 1x, 10 fps = game speed, 640x360 and 640x400) with the window-vs-headless hash
#      check of scripts/room_capture.sh, generated twice and compared byte for byte;
#   3. five at-speed ability GIFs for the abilities whose committed GIFs are slowed down or faster than
#      game speed (chain, reflex, Magic Missile, Light on Your Feet, Puddle Jumper), each checked
#      window against headless, each generated twice and compared;
#   4. one PLAYTRACE session of room 6 (scripted pilot, real window), verified strict;
#   5. the scorecard (scorecard.md): the committed scripted pilots at seeds 0..8, the PT-TUNE D1 to D6
#      figures and the CA2 metrics M1 to M16 of the arena, plus room 4's single room-pilot run.
# Perf is scripts/make_f01_gate_b_perf.sh (run on its own on a quiet machine).
# Needs the debug binaries (cargo build --locked --bin floor01_scene --bin playtrace), the exported
# atlases (bash scripts/export_assets.sh), xvfb-run, Pillow and cargo (the scorecard is the ignored test
# `f01_gate_scorecard::tests::prints_the_gate_scorecard`; use the same CARGO_TARGET_DIR as the build).
# The README is hand-written. Never records a human: the session is a scripted pilot.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   PT=<playtrace> (default ./target/debug/playtrace)
#   CAPTURES, SESSIONS: scratch dirs, which must be under artifacts/ (they are deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
pt="${PT:-./target/debug/playtrace}"
caps="${CAPTURES:-artifacts/f01-playtest-captures-b}"
sessions="${SESSIONS:-artifacts/f01-playtest-sessions-b}"
for dir in "$caps" "$sessions"; do
  case "$dir" in
    artifacts/?*) ;;
    *) echo "refusing scratch dir '$dir': it must be a path under artifacts/" >&2; exit 2 ;;
  esac
done
out=docs/evidence/f01-playtest
rm -rf "$caps" "$sessions"
mkdir -p "$caps" "$sessions" "$out"
rm -f "$out"/wave-*.gif "$out"/ability-*.gif "$out"/capture_lines_b.txt "$out"/pilot_hashes_mastery.txt \
  "$out"/playtrace_session_mastery.txt "$out"/scorecard.md
xvfb() { xvfb-run -a -s '-screen 0 1280x720x24' "$@"; }
last_state() { grep '^COMBAT_STATE ' "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }

# 1. The mastery room pilot's final state (the helper exits non-zero unless two runs are identical).
python3 scripts/room_pilot_hashes.py mastery mastery > "$out/pilot_hashes_mastery.txt"

# 2 and 3. The GIFs, two passes.
: > "$out/capture_lines_b.txt"
fixture_gif() { # pass name fixture first:step:count step_ticks
  local pass="$1" name="$2" fx="$3" seq="$4" step="$5" dir="$caps/$1/$2"
  local last first count
  IFS=: read -r first _ count <<<"$seq"
  last=$((first + step * (count - 1)))
  mkdir -p "$dir"
  xvfb "$BIN" --scale 1 --party-fixture "$fx" --capture-sequence "$seq" --capture-dir "$dir" > "$dir/run.log" 2>&1
  "$BIN" --headless-floor01-smoke --party-fixture "$fx" --capture-tick "$last" > "$dir/headless.log" 2>&1
  local w h match=no
  w="$(last_state "$dir/run.log")"; h="$(last_state "$dir/headless.log")"
  [[ -n "$w" && "$w" == "$h" ]] && match=yes
  [[ "$pass" == a ]] && echo "GATE_B_CAPTURE name=$name fixture=$fx first_tick=$first step=$step last_tick=$last window_hash=${w:-none} headless_hash=${h:-none} match=$match" >> "$out/capture_lines_b.txt"
  [[ "$match" == yes ]]
  python3 scripts/room_gif.py "$dir" "$caps/$pass/$name.gif" --step-ticks "$step"
}
for pass in a b; do
  mkdir -p "$caps/$pass"
  # The mastery arena's second wave, W2 (the llama yard): ticks 560 to 854, 6 ticks a frame.
  bash scripts/room_capture.sh mastery mastery 560:6:50 "$caps/$pass/wave-w2" \
    | { [[ "$pass" == a ]] && sed 's/^/GATE_B_WAVE /' >> "$out/capture_lines_b.txt" || cat > /dev/null; }
  python3 scripts/room_gif.py "$caps/$pass/wave-w2" "$caps/$pass/wave-mastery-w2.gif" --step-ticks 6
  # The same wave in the 640x400 deck view.
  bash scripts/room_capture.sh mastery mastery 560:6:50 "$caps/$pass/wave-w2-deck" --deck \
    | { [[ "$pass" == a ]] && sed 's/^/GATE_B_WAVE_640x400 /' >> "$out/capture_lines_b.txt" || cat > /dev/null; }
  python3 scripts/room_gif.py "$caps/$pass/wave-w2-deck" "$caps/$pass/wave-mastery-w2-640x400.gif" --step-ticks 6
  fixture_gif "$pass" ability-chain carl_prog_dyn_chain_depth_cap 188:3:20 3
  fixture_gif "$pass" ability-reflex cf02_dm_reflex_cues 712:3:30 3
  fixture_gif "$pass" ability-magic-missile cf02_assist_donut_missiles_goblin 8:2:32 2
  fixture_gif "$pass" ability-light-on-your-feet donut_prog_arena_ledge_boost 28:2:37 2
  fixture_gif "$pass" ability-puddle-jumper donut_prog_arena_blink_tour 30:2:40 2
done
for g in wave-mastery-w2 wave-mastery-w2-640x400 ability-chain ability-reflex ability-magic-missile ability-light-on-your-feet ability-puddle-jumper; do
  cmp "$caps/a/$g.gif" "$caps/b/$g.gif"
  cp "$caps/a/$g.gif" "$out/$g.gif"
done
echo "GATE_B_BYTE_STABLE gifs=7 passes=2 result=identical" >> "$out/capture_lines_b.txt"

# 4. One recorded session of room 6, scripted pilot, real window, verified with the same source.
dir="$sessions/mastery"; log="$sessions/mastery.log"; mkdir -p "$dir"
xvfb "$BIN" --scale 1 --no-audio --scene f01_late --f01-room mastery --room-pilot mastery \
  --record --record-dir "$dir" --exit-after-ticks 1300 > "$log" 2>&1
id="$(ls "$dir")"
{
  echo "== room=mastery plan=mastery label=mastery exit_after_ticks=1300"
  grep '^PLAYTRACE_SAVED ' "$log" | sed -E 's/^(PLAYTRACE_SAVED) session=[0-9a-f]+ (.*)/\1 \2/'
  "$pt" verify "$id" --dir "$dir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
  "$pt" inspect "$id" --dir "$dir" | grep -E '^  (schema|reset|ticks|episodes)' | sed 's/^/inspect:/'
  echo "journal_capability_granted_count=$(grep -h -o '"k":"CapabilityGranted"' "$dir/$id"/ticks-*.jsonl | wc -l)"
} > "$out/playtrace_session_mastery.txt"
grep -q 'result=PASS scope=full' "$out/playtrace_session_mastery.txt"

# 5. The scorecard: the ignored generator test, then room 4's room-pilot run.
cargo test --locked --lib f01_gate_scorecard -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | sed -n '/^SCORECARD_BEGIN$/,/^SCORECARD_END$/p' | sed '1d;$d' > "$out/scorecard.md"
[[ -s "$out/scorecard.md" ]]
hz="$("$BIN" --headless-floor01-smoke --scene f01_late --f01-room hazards --room-pilot direct --capture-tick 1500 2>&1)"
count() { grep -c "kind=$1" <<<"$hz" || true; }
{
  echo
  echo "## Room 4, the hazards room (the \`direct\` room pilot, one run to tick 1500; no per-seed pilots exist)"
  echo
  echo "| Pilot | Hero defeats | Respawns | Hurts | Blasts | Rot stickers woken | Enemies left | Final state |"
  echo "| --- | --- | --- | --- | --- | --- | --- | --- |"
  state="$(grep '^COMBAT_STATE ' <<<"$hz" | tail -n 1 | sed -E 's/.* hero_hp=([0-9]+) deaths=([0-9]+) enemies=.* hash=([0-9a-f]+) .*/hp \1, deaths \2, hash \3/')"
  left="$(grep '^F01_HAZARDS_SMOKE_OK ' <<<"$hz" | sed -E 's/.* enemies=([0-9]+) .*/\1/')"
  echo "| direct (solo Carl, plus the follower bot) | $(count hero_defeated) | $(count respawn) | $(count hero_hurt) | $(count blast) | $(count rot_provoked) | ${left:-NOT RUN} | $state |"
  echo
  echo "The room pilot walks into the pockets and is defeated and retries; it does not dodge hazards (it is a filming pilot, not a player). Solo Donut by swap and the companion have no per-seed pilot in this room: NOT RUN."
} >> "$out/scorecard.md"

(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
