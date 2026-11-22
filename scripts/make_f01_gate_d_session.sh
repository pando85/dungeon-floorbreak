#!/usr/bin/env bash
# PT-GATE-D (plan section 10, items 2 and 4c): one PLAYTRACE session of room 6 on the human default path
# (`--scene f01_late --f01-room mastery`, no `--room-pilot`: the live `cf02_free` partner), verified with
# the PLAYTRACE verifier into docs/evidence/f01-playtest/playtrace_session_default_path.txt.
# No human, no pilot and no synthetic window input (no xdotool): Carl gets no input (an idle P1; the
# partner plays). FIX-TOOL: the window ends itself with `--exit-after-ticks 1200` (a scripted exit that keeps
# the live input source; it needs no pilot), so the session is closed cleanly and verifies as
# `scope=full`; `playtrace inspect` labels it input-free, not as a person playing.
# Needs the debug binaries (cargo build --locked --bin floor01_scene --bin playtrace), the exported atlases
# (bash scripts/export_assets.sh) and xvfb-run.
#   BIN=<floor01_scene> PT=<playtrace>   SESSIONS: a scratch dir under artifacts/ (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
pt="${PT:-./target/debug/playtrace}"
sessions="${SESSIONS:-artifacts/f01-gate-d-sessions}"
case "$sessions" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$sessions': it must be a path under artifacts/" >&2; exit 2 ;;
esac
out=docs/evidence/f01-playtest
rm -rf "$sessions"
mkdir -p "$sessions" "$out"
rm -f "$out/playtrace_session_default_path.txt"
dir="$sessions/default"; log="$sessions/default.log"; mkdir -p "$dir"
xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 --no-audio --scene f01_late --f01-room mastery \
  --record --record-dir "$dir" --exit-after-ticks 1200 > "$log" 2>&1
id="$(ls "$dir")"
{
  echo "== room=mastery scene=f01_late no --room-pilot, no input (an idle P1, the live partner), ended by --exit-after-ticks 1200 (a clean close: full scope)"
  grep -E '^(F01_LATE_ENTRY|COMPANION_CONFIG)' "$log" | sed -E 's/^(F01_LATE_ENTRY room=[a-z]+).*(loadout=[a-z]+)$/\1 \2/'
  grep '^PLAYTRACE_SAVED ' "$log" | sed -E 's/^(PLAYTRACE_SAVED) session=[0-9a-f]+ (.*)/\1 \2/'
  "$pt" verify "$id" --dir "$dir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
  "$pt" inspect "$id" --dir "$dir" | grep -E '^  (status|schema|reset|ticks|episodes)' | sed -E 's/^  /inspect:/; s/ build .*//'
} > "$out/playtrace_session_default_path.txt"
grep -q 'COMPANION_CONFIG: set=cf02_free' "$out/playtrace_session_default_path.txt"
grep -q 'result=PASS scope=full' "$out/playtrace_session_default_path.txt"
grep -q 'run_kind Human (input-free' "$out/playtrace_session_default_path.txt"


(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
