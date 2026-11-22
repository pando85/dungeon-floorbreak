#!/usr/bin/env bash
# PT-GATE-D (plan section 10, items 2 and 4c): one PLAYTRACE session of room 6 on the human default path
# (`--scene f01_late --f01-room mastery`, no `--room-pilot`: the live `cf02_free` partner), verified with
# the PLAYTRACE verifier into docs/evidence/f01-playtest/playtrace_session_default_path.txt.
# No human, no pilot and no synthetic window input (no xdotool): Carl gets no input (an idle P1; the
# partner plays). Without a pilot or scenario the build has no scripted exit (`--exit-after-ticks` needs
# one) and its SIGINT does not end the window, so the window is ended with one SIGTERM after 20 s: the
# session is crash-recovered and verifies as `scope=prefix`, NOT `scope=full`.
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
xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 --no-audio --scene f01_late --f01-room mastery --record --record-dir "$dir" > "$log" 2>&1 &
xvfb_pid=$!
app_pid=""
for _ in $(seq 1 60); do
  app_pid="$(pgrep -f -- "--record-dir $dir" | grep -v "^$xvfb_pid$" | while read -r p; do
    [[ "$(cat "/proc/$p/comm" 2>/dev/null)" == floor01_scene ]] && echo "$p"; done | head -n 1 || true)"
  [[ -n "$app_pid" ]] && break
  sleep 1
done
[[ -n "$app_pid" ]] || { echo "the window did not start" >&2; exit 1; }
sleep 20
kill -TERM "$app_pid"   # the only way out without input: a crash-recovered session (verifies as scope=prefix)
wait "$xvfb_pid" || true
id="$(ls "$dir")"
{
  echo "== room=mastery scene=f01_late no --room-pilot, no input (an idle P1, the live partner), ended by one SIGTERM after 20 s (crash-recovered: prefix scope)"
  grep -E '^(F01_LATE_ENTRY|COMPANION_CONFIG)' "$log" | sed -E 's/^(F01_LATE_ENTRY room=[a-z]+).*(loadout=[a-z]+)$/\1 \2/'
  echo "PLAYTRACE_SAVED: none (the app was killed; the session was crash-recovered)"
  "$pt" verify "$id" --dir "$dir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
  "$pt" inspect "$id" --dir "$dir" | grep -E '^  (status|schema|reset|ticks|episodes)' | sed -E 's/^  /inspect:/; s/ build .*//'
} > "$out/playtrace_session_default_path.txt"
grep -q 'COMPANION_CONFIG: set=cf02_free' "$out/playtrace_session_default_path.txt"
grep -q 'result=PASS scope=prefix' "$out/playtrace_session_default_path.txt"


(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
