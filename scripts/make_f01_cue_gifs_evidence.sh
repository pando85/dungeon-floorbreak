#!/usr/bin/env bash
# FA-Q38 (i-frame cue) regeneration of the four owner-packet GIFs in docs/evidence/f01-playtest/ in which a
# hero is hurt, so that the committed clips show the cue:
#   wave-mastery-w2.gif and wave-mastery-w2-640x400.gif   (the wave step of scripts/make_f01_gate_b_evidence.sh:
#       `--room-pilot mastery`, ticks 440 to 734, 6 ticks a frame; Carl is hurt on tick 701);
#   route-room4-direct.gif and route-room4-direct-640x400.gif   (scripts/make_f01_gate_e_evidence.sh, run as is:
#       `--room-pilot hazards:carl_solo:0`, ticks 320 to 650; Carl is hurt on ticks 357 and 626).
# Only the wave step of gate B is repeated (the rest of gate B, the ability GIFs, the recorded session and the
# scorecard, does not show a hurt hero and is not rerun). Every GIF is generated twice and compared byte for
# byte; each window run must end on its headless hash, and the two GATE_B_WAVE lines of capture_lines_b.txt
# must come out unchanged (the cue is presentation only). Gate E also rewrites the tactical GIFs: they must come
# out byte-identical to the committed ones (check `git status` afterwards: only the four GIFs, and
# SHA256SUMS, may change). Windows run one after the other under xvfb (a GPU window capture: run it on a quiet
# machine).
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   CAPTURES: a scratch dir under artifacts/ (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/f01-cue-gifs}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
out=docs/evidence/f01-playtest
rm -rf "$caps"
mkdir -p "$caps"
lines="$caps/wave_lines.txt"
: > "$lines"
for pass in a b; do
  mkdir -p "$caps/$pass"
  bash scripts/room_capture.sh mastery mastery 440:6:50 "$caps/$pass/wave-w2" \
    | { [[ "$pass" == a ]] && sed 's/^/GATE_B_WAVE /' >> "$lines" || cat > /dev/null; }
  python3 scripts/room_gif.py "$caps/$pass/wave-w2" "$caps/$pass/wave-mastery-w2.gif" --step-ticks 6
  bash scripts/room_capture.sh mastery mastery 440:6:50 "$caps/$pass/wave-w2-deck" --deck \
    | { [[ "$pass" == a ]] && sed 's/^/GATE_B_WAVE_640x400 /' >> "$lines" || cat > /dev/null; }
  python3 scripts/room_gif.py "$caps/$pass/wave-w2-deck" "$caps/$pass/wave-mastery-w2-640x400.gif" --step-ticks 6
done
# The capture lines (window and headless hashes) must not move: the two lines are replaced by themselves.
python3 - "$out/capture_lines_b.txt" "$lines" <<'PY'
import sys
path, new = sys.argv[1], sys.argv[2]
fresh = {l.split(" ", 1)[0]: l for l in open(new).read().splitlines() if l}
old = open(path).read().splitlines()
out = [fresh.pop(l.split(" ", 1)[0], l) if l.split(" ", 1)[0] in ("GATE_B_WAVE", "GATE_B_WAVE_640x400") else l for l in old]
assert not fresh, fresh
open(path, "w").write("\n".join(out) + "\n")
PY
grep -q 'match=no' "$lines" && { echo "a window run differs from its headless run" >&2; exit 1; }
for g in wave-mastery-w2 wave-mastery-w2-640x400; do
  cmp "$caps/a/$g.gif" "$caps/b/$g.gif"
  cp "$caps/a/$g.gif" "$out/$g.gif"
done
# The direct route (and the tactical one, which must not change), two passes each, and the SHA256SUMS.
CAPTURES="$caps/gate-e" bash scripts/make_f01_gate_e_evidence.sh
