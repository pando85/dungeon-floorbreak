#!/usr/bin/env bash
# Build the COMBAT-INT-01 evidence from the real Bevy capture that
# `scripts/ci_floor01_combat.sh render` writes to artifacts/combat-int-01.
#   bash scripts/make_combat_int_evidence.sh
# Needs ImageMagick (magick or convert). Writes docs/evidence/combat-int-01:
# the three stills (hit tick 125, stagger tick 129, defeat tick 142), the GIF
# (ticks 118..178, one frame per tick at half size), the headless replay logs
# and SHA256SUMS. The README is hand-written.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
frames=artifacts/combat-int-01/frames_cp01_single_goblin
out=docs/evidence/combat-int-01
fixtures=(cp01_single_goblin cp01_two_hits cp02_two_goblins)
if command -v magick >/dev/null 2>&1; then im=(magick); else im=(convert); fi
[[ -d "$frames" ]] || { echo "run scripts/ci_floor01_combat.sh render first" >&2; exit 1; }
mkdir -p "$out"
still() { # name tick
  local file
  file="$(printf '%s/frame_%04d_tick_%05d.png' "$frames" "$(($2 - 118))" "$2")"
  cp "$file" "$out/cp01_single_goblin_$1_tick$2_1920x1080.png"
}
still hit 125
still stagger 129
still defeated 142
"${im[@]}" -delay 2 -loop 0 "$frames"/frame_*.png -resize 50% -layers Optimize \
  "$out/cp01_single_goblin_ticks118-178_960x540.gif"
for f in "${fixtures[@]}"; do
  ./target/debug/floor01_scene --headless-floor01-smoke --fixture "$f" \
    | grep -E '^(COMBAT_EVENT|COMBAT_STATE)' > "$out/headless_${f}.txt"
done
(cd "$out" && find . -maxdepth 1 -type f ! -name SHA256SUMS ! -name README.md -printf '%P\n' | sort | xargs sha256sum > SHA256SUMS)
ls -l "$out"
