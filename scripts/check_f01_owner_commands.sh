#!/usr/bin/env bash
# PT-GATE-B: runs every owner launch command of docs/evidence/f01-playtest/README.md (plan section 9)
# in the real window under xvfb for a short run (`--exit-after-ticks 30`, no audio, no recording) and
# writes one line per command into docs/evidence/f01-playtest/owner_commands_check.txt. A command the
# game refuses (an unlisted room) is a line `refused`, not a failure. It proves the arguments parse and
# the scene starts on this head; it is not a play-through, and it runs the debug binary (the owner's
# commands use `cargo run --release --bin floor01_scene -- ARGS`, which takes the same arguments).
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   EVAL=<corral_eval> (default ./target/debug/corral_eval)
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out=docs/evidence/f01-playtest/owner_commands_check.txt
: > "$out"
fail=0
# Logs of unexpected outcomes (never committed; target/ is ignored).
keep="${OWNER_LOG_DIR:-target/owner_command_logs}"
try() { # expectation(ok|refuse) args...
  local want="$1"; shift
  local log code
  log="$(mktemp -t owner_cmd.XXXXXX)"
  timeout 120 xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --no-audio --no-record "$@" --exit-after-ticks 30 > "$log" 2>&1
  code=$?
  local res
  if [[ "$code" == 0 ]]; then res=ok; else res="refused (exit $code): $(grep -v '^\s*$' "$log" | tail -n 1 | cut -c1-160)"; fi
  echo "[$want] $res :: floor01_scene $*" >> "$out"
  case "$want:$res" in
    ok:ok|refuse:refused*) ;;
    *)
      # Review F6 of #271: keep the full log of an unexpected outcome.
      mkdir -p "$keep"
      cp "$log" "$keep/$(date +%s)-$$-${RANDOM}.log"
      echo "  (full log kept under $keep)" >> "$out"
      fail=1
      ;;
  esac
  rm -f "$log"
}
S="--scene f01_late --scale 2"
try ok $S
# F01-RUN: the whole floor in one session (the first listed room; a 30-tick run never reaches an exit).
try ok $S --f01-run
try ok $S --f01-run --f01-loadout all
try refuse $S --f01-run --cf02-seed 3
# AREA-RINGS-01: a floor run can start in either Ring room (the kit renders through the one launch path), and
# `--ring-pilot` (the evidence pilot of the Ring rooms) is refused outside them.
for room in ring_court ring_split; do try ok $S --f01-run --f01-room "$room"; done
try refuse $S --f01-room hoarder --ring-pilot
# CORRAL-F: room 0 (the corral) is the first listed room; DR-A lists room 3 (the dozer).
for room in corral hoarder boiler dozer hazards ring_court ring_split juicer mastery; do try ok $S --f01-room "$room"; done
# No committed room is unlisted any more: an unknown id is refused.
try refuse $S --f01-room cellar
# RING-A/B wire: the Ring rooms launch only through f01_late (KR22: no scene of their own).
try refuse --scene f01_ring_court --scale 2
# FA-Q24: the Hoarder is listed again (it reverses PTG-AD1); her own scene still launches her too.
try ok --scene f01_hoarder --scale 2
# DR-A: room 3 (the Dozer revenge) is listed, and its own scene launches it too.
try ok --scene f01_dozer --scale 2
try ok $S --f01-room mastery --f01-loadout all
try ok $S --f01-room hoarder --f01-loadout all
try ok $S --f01-room hoarder --companion cf02_assist
try ok $S --f01-room boiler --f01-loadout story
# Plan sections 8 and 9: a room scene defaults the AI partner to cf02_free and --companion overrides it
# (PT-GATE-C1); the placeholder fixture a room scene sets is not a user fixture.
try ok $S --f01-room mastery --companion cf02_free
try ok $S --f01-room boiler --companion cf02_assist
try ok $S --f01-room hazards --companion cf02_free
try ok $S --f01-room hazards --companion off
# FA-Q08: room 5 (the Juicer) takes --companion like rooms 2, 4 and 6; off restores the follow bot.
try ok $S --f01-room juicer --companion cf02_free
try ok $S --f01-room juicer --companion off
try refuse $S --f01-room mastery --fixture cp01_kick --companion cf02_free
try ok $S --f01-room boiler --debug-combat
try ok --cf02 --cf02-engineer --donut-biscuit --scale 2
try ok --cf02 --cf02-engineer --donut-biscuit --carl-dynamite --scale 2
try ok --cf02 --cf02-engineer --donut-biscuit --carl-dynamite --companion cf02_free --scale 2
try ok --cf02 --cf02-engineer --carl-dynamite --carl-gauntlet --donut-puddle --scale 2
try ok --scene boss_f01 --carl-prog --donut-prog --scale 2
try ok --scene f01_boiler --scale 2
evaldir="$(mktemp -d -t corral_eval.XXXXXX)"
if timeout 300 "${EVAL:-./target/debug/corral_eval}" --out "$evaldir" > /dev/null 2>&1; then
  echo "[ok] ok :: corral_eval --out DIR" >> "$out"
else
  echo "[ok] FAILED :: corral_eval --out DIR" >> "$out"; fail=1
fi
rm -rf "$evaldir"
[[ -x tools/record-run.sh ]] && echo "[exists] tools/record-run.sh is executable (the window recording itself was NOT RUN: needs a display, xdotool, ffmpeg and a human)" >> "$out"
(cd docs/evidence/f01-playtest && sha256sum ./*.gif ./*.png ./*.txt ./*.md 2>/dev/null | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
exit "$fail"
