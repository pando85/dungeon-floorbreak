#!/usr/bin/env bash
# PLAYTRACE-01 isolation check (PT-Q12): scripted, headless, capture and CI
# paths must not create a recording directory, and --record must.
#
#   scripts/check_playtrace_isolation.sh [--with-ci-script]
#
# Runs with a temporary XDG_DATA_HOME and HOME, so the real data directory is
# never touched. Needs ./target/debug/floor01_scene and ./target/debug/playtrace
# (cargo build --bins). The windowed parts need xvfb-run and generated/art
# (bash scripts/export_assets.sh); without them they print SKIP, never PASS.
# --with-ci-script also runs `ci_floor01_combat.sh headless` under the same
# empty environment.
set -euo pipefail
bin=./target/debug/floor01_scene
tool=./target/debug/playtrace
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export XDG_DATA_HOME="$tmp/xdg" HOME="$tmp/home"
mkdir -p "$HOME"
unset CI FLOORBREAK_RECORD

no_recordings() {
  if [[ -e "$XDG_DATA_HOME/dungeon-floorbreak" ]]; then
    echo "FAIL: $1 created $XDG_DATA_HOME/dungeon-floorbreak" >&2
    find "$XDG_DATA_HOME" -maxdepth 4 >&2
    exit 1
  fi
  echo "PASS: $1 wrote no recording"
}

"$bin" --headless-floor01-smoke --scenario walk_right --capture-tick 80 >/dev/null
no_recordings "headless scenario capture-tick"
"$bin" --headless-floor01-smoke --fixture cp01_single_goblin >/dev/null
no_recordings "headless fixture"
"$bin" --headless-floor01-smoke --party-fixture duo_independent_moves >/dev/null
no_recordings "headless party fixture"
"$bin" --headless-floor01-smoke --demo --autopilot >/dev/null
no_recordings "headless demo autopilot"
"$bin" --list-fixtures cp01 >/dev/null
no_recordings "list-fixtures"

if [[ "${1:-}" == "--with-ci-script" ]]; then
  scripts/ci_floor01_combat.sh headless >/dev/null
  no_recordings "scripts/ci_floor01_combat.sh headless"
fi

if command -v xvfb-run >/dev/null && [[ -d generated/art ]]; then
  xvfb-run -a -s '-screen 0 1280x800x24' "$bin" --no-audio --fixture cp01_single_goblin --exit-after-ticks 30 >/dev/null
  no_recordings "windowed scripted fixture with --exit-after-ticks"
  xvfb-run -a -s '-screen 0 1280x800x24' "$bin" --no-audio --scenario walk_right --exit-after-ticks 30 >/dev/null
  no_recordings "windowed scripted scenario with --exit-after-ticks"
  # A live-input run in CI (CI is set) records nothing either; run it briefly.
  CI=true timeout 8 xvfb-run -a -s '-screen 0 1280x800x24' "$bin" --no-audio --scale 1 >/dev/null 2>&1 || true
  no_recordings "windowed live story run with CI=true"
  FLOORBREAK_RECORD=off timeout 8 xvfb-run -a -s '-screen 0 1280x800x24' "$bin" --no-audio --scale 1 >/dev/null 2>&1 || true
  no_recordings "windowed live story run with FLOORBREAK_RECORD=off"
  # Positive control: --record forces a session, closed cleanly at app exit.
  xvfb-run -a -s '-screen 0 1280x800x24' "$bin" --no-audio --record --fixture cp01_single_goblin --exit-after-ticks 30 | grep '^PLAYTRACE_'
  "$tool" list
  id="$("$tool" list | awk 'NR==2 {print $1}')"
  "$tool" inspect "$id"
  "$tool" inspect "$id" | grep -q 'status Completed'
  echo "PASS: --record wrote and closed one session"
else
  echo "SKIP: windowed checks (need xvfb-run and generated/art)"
fi
