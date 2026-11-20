#!/usr/bin/env bash
# CF02-C2/F: the real Bevy render loop reproduces the headless replay.
#
#   xvfb-run -a -s '-screen 0 1280x720x24' bash scripts/cf02_render_match.sh
#
# For each room, swap and bail-out fixture the party fixture is rendered (a capture
# sequence of one frame at the fixture's last tick) and its COMBAT_EVENT lines and
# final COMBAT_STATE line must equal the headless replay to that tick; then the live
# `--cf02` room (autopilot, assistant off) is rendered for 487 ticks and must equal its
# headless replay. Needs a debug build and the exported atlases.
set -euo pipefail
export FLOORBREAK_RECORD=off
bin=./target/debug/floor01_scene
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
lines() { grep -E '^(COMBAT_EVENT|COMBAT_STATE)' | awk '/^COMBAT_STATE/ { last = $0; next } { print } END { print last }'; }
fixtures=$({ "$bin" --list-fixtures cf02_arena_; "$bin" --list-fixtures cf02_block_; \
  printf '%s\n' cf02_goblin_self_own cf02_intact_shore_ignores_goblins cf02_parapet_cover \
  cf02_perch_slab_climb cf02_perch_above_spit cf02_perch_dive cf02_donut_bailout cf02_swap_eased_pan; } | sort -u)
for f in $fixtures; do
  # The capture tick counts every simulated tick over the attempts (a restart sets the
  # attempt-local tick back to 0; `elapsed` does not).
  last="$("$bin" --headless-floor01-smoke --party-fixture "$f" | grep '^COMBAT_STATE' | tail -n 1 | sed -n 's/.* elapsed=\([0-9]*\) .*/\1/p')"
  "$bin" --headless-floor01-smoke --party-fixture "$f" --capture-tick "$last" | lines > "$work/head.txt"
  "$bin" --scale 1 --party-fixture "$f" --capture-sequence "${last}:1:1" --capture-dir "$work/frames" > "$work/render.log" 2>&1
  grep -q 'FLOOR1_CAPTURE_SEQUENCE_DONE count=1' "$work/render.log"
  lines < "$work/render.log" > "$work/render.txt"
  if ! diff "$work/head.txt" "$work/render.txt" > /dev/null; then
    diff "$work/head.txt" "$work/render.txt" | head -10 >&2 || true
    echo "render differs from headless: $f" >&2
    exit 1
  fi
  echo "CF02_RENDER_MATCH: $f tick $last"
  rm -rf "$work/frames"
done
ticks=487
"$bin" --headless-floor01-smoke --cf02 --companion off --autopilot --capture-tick "$ticks" | lines > "$work/head.txt"
"$bin" --scale 3 --cf02 --companion off --autopilot --exit-after-ticks "$ticks" > "$work/live.log" 2>&1
lines < "$work/live.log" > "$work/live.txt"
diff "$work/head.txt" "$work/live.txt"
grep -q '^CF02_ARENA_COMPLETE' "$work/live.log"
echo "CF02_RENDER_MATCH: live --cf02 room, autopilot, $ticks ticks, CF02_ARENA_COMPLETE"
