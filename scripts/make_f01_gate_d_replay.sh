#!/usr/bin/env bash
# PT-GATE-D and PT-GATE-E (plan section 10, item 4b): the render-loop replay of every aligned room pilot.
#
# Every (who, seed) of the Juicer's room (`juicer:WHO:SEED`, 3 who x 9 seeds = 27), of the closing arena
# (`mastery:WHO:SEED`, 4 who x 9 seeds = 36), of the hazards room (`hazards:WHO:SEED`, 3 who x 9 seeds = 27,
# PT-GATE-E) and of the Boiler's live room (`boiler:WHO:direct|both:SEED`, 3 who x 2 plans x 9 seeds = 54,
# PT-GATE-E) is run through scripts/room_capture.sh to the tick the library fixture ends on: a real window under xvfb (the Bevy presentation) and the
# headless smoke, twice. For each run the window's final COMBAT_STATE hash must equal the headless
# one, and both must equal the library fixture's: the combat hash of the same state (printed by the
# ignored test `room_pilots::tests::prints_the_aligned_library_hashes`) and, from the headless
# ROOM_PILOT_STATE line, the library fixture's final hash itself. Output: render_replay_lines.txt,
# one RENDER_REPLAY line per run and a closing RENDER_REPLAY_TOTAL line. The script fails unless
# all 144 match. (The 9 Rot-off hazards runs of #240 build the real `hazq02_rot_enabled` switch since FIX-TOOL,
# but the shipped scene has no flag that turns it off: they have no scene route to replay and are not in this set.)
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run and cargo (the library test; use the same
# CARGO_TARGET_DIR as the build). The windows run one after the other (one display at a time; room_capture.sh
# tolerates xvfb-run's cleanup exit status, but a parallel run is not supported).
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<lines file> (default docs/evidence/f01-playtest/render_replay_lines.txt)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   ROOMS="boiler hazards" (optional): run only those rooms and keep the committed lines of the others
#     (a partial re-run after an output-only change of one room's smoke; the closing total covers all 144)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-playtest/render_replay_lines.txt}"
caps="${CAPTURES:-artifacts/f01-gate-d-replay}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
rm -rf "$caps"
mkdir -p "$caps" "$(dirname "$out")"
library="$caps/library.txt"
cargo test --locked --lib room_pilots::tests::prints_the_aligned_library_hashes -- --ignored --nocapture --test-threads=1 2>/dev/null \
  | grep '^ALIGNED_LIBRARY room=' > "$library"
[[ "$(wc -l < "$library")" -eq 144 ]] || { echo "expected 144 library lines" >&2; exit 1; }
field() { sed -E "s/.* $1=([^ ]*).*/\\1/" <<<"$2"; }
: > "$out.tmp"
total=0 matched=0
while read -r line; do
  room="$(field room "$line")" who="$(field who "$line")" seed="$(field seed "$line")"
  ticks="$(field ticks "$line")" lib="$(field hash "$line")" libc="$(field combat_hash "$line")"
  route=""
  case "$room" in
    boiler) route="$(field route "$line")"; plan="$room:$who:$route:$seed"; tag="$room-$who-$route-$seed" ;;
    *) plan="$room:$who:$seed"; tag="$room-$who-$seed" ;;
  esac
  if [[ -n "${ROOMS:-}" && " $ROOMS " != *" $room "* ]]; then
    # A kept line is looked up by its full key (the Boiler's `route=` sits between `who=` and `seed=`) and must
    # still agree with the fresh library line: same tick, library hash and combat hash, and a `match=yes` result.
    kept="$(grep -F "RENDER_REPLAY room=$room who=$who${route:+ route=$route} seed=$seed " "$out" | head -n 1 || true)"
    [[ -n "$kept" ]] || { echo "no committed line to keep for $plan" >&2; exit 1; }
    if [[ "$(field tick "$kept")" != "$ticks" || "$(field library_hash "$kept")" != "$lib" \
      || "$(field library_combat_hash "$kept")" != "$libc" || "$kept" != *" match=yes" ]]; then
      echo "the kept line for $plan is stale against the fresh library (tick, library_hash, library_combat_hash or match): re-run its room" >&2
      exit 1
    fi
    echo "$kept" >> "$out.tmp"
    total=$((total + 1))
    matched=$((matched + 1))
    continue
  fi
  window=() headless=() pilot_hash="" pilot_tick="" ok=yes
  for run in 1 2; do
    dir="$caps/$tag-run$run"
    # One frame, on the library fixture's final tick: first=ticks, step=1, count=1.
    bash scripts/room_capture.sh "$room" "$plan" "$ticks:1:1" "$dir" > /dev/null || ok=no
    state() { grep '^COMBAT_STATE ' "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
    window+=("$(state "$dir/run.log")")
    headless+=("$(state "$dir/headless.log")")
    pstate="$(grep '^ROOM_PILOT_STATE ' "$dir/headless.log" | tail -n 1 || true)"
    pilot_hash+="$(field hash "$pstate") "
    pilot_tick+="$(field tick "$pstate") "
  done
  [[ "${window[0]}" == "${window[1]}" && "${headless[0]}" == "${headless[1]}" ]] || ok=no
  [[ "${window[0]}" == "${headless[0]}" && "${window[0]}" == "$libc" ]] || ok=no
  read -r ph1 ph2 <<<"$pilot_hash"
  read -r pt1 pt2 <<<"$pilot_tick"
  [[ "$ph1" == "$lib" && "$ph2" == "$lib" && "$pt1" == "$ticks" && "$pt2" == "$ticks" ]] || ok=no
  echo "RENDER_REPLAY room=$room who=$who${route:+ route=$route} seed=$seed tick=$ticks library_hash=$lib library_combat_hash=$libc window_hash_run1=${window[0]:-none} window_hash_run2=${window[1]:-none} headless_hash_run1=${headless[0]:-none} headless_hash_run2=${headless[1]:-none} pilot_state_hash=${ph1:-none} match=$ok" >> "$out.tmp"
  total=$((total + 1))
  [[ "$ok" == yes ]] && matched=$((matched + 1))
done < "$library"
echo "RENDER_REPLAY_TOTAL runs=$total matched=$matched juicer=27 arena=36 hazards=27 boiler=54 passes=2" >> "$out.tmp"
mv "$out.tmp" "$out"
[[ "$total" -eq 144 && "$matched" -eq 144 ]]
