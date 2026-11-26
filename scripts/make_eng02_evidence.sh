#!/usr/bin/env bash
# ENG-02 evidence: real Bevy captures of the Goblin Engineer's get-up, ram run and finishes.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_eng02_evidence.sh [out_dir]
#
# Every clip is captured twice with scripts/room_capture.sh (a real window under xvfb plus the headless run of the
# same tick, whose final COMBAT_STATE hashes must be equal) and the two generations must be byte-identical frame
# for frame. The rooms are the f01_late room scenes (the `--f01-run` rooms: room 2, the Goblin Boiler, and room 6,
# the mastery arena), played by their aligned room pilots. Those pilots only ever kill the Engineer with a jab (the
# knockback finish), and never let his ram run for more than a tick, so:
#   * the finishes are forced with the presentation-only lab flag `--death-lab-engineer NAME` (the simulation, its
#     events and its hash are untouched: window hash equals headless hash, with or without the flag);
#   * the ram run is captured in the CF02 arena fixture `eng_jump_over_ram`, where the ram passes under Carl's jump;
#   * the get-up (the Engineer knocked down by a dive stomp) and the knockback finish are natural.
# Output: stills (3x crops) and GIFs (game speed) under OUT, `capture_lines.txt` (one ENG02_CAPTURE line per clip)
# and the clip names. Needs xvfb-run, flock, python3 with Pillow and a debug build.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
out="${1:-docs/evidence/eng-02/in-scene}"
caps="${CAPTURES:-artifacts/eng02-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
export BIN="${BIN:-./target/debug/floor01_scene}"
export FLOORBREAK_RECORD=off
rm -rf "$caps"
mkdir -p "$caps" "$out"
lines="$out/capture_lines.txt"
: > "$lines"

R2="boiler boiler:carl_solo:direct:0"      # room 2: the Engineer is knocked down at 354 and 369, gets up at 406 and dies to a jab at 792
R6="mastery mastery:carl_solo:0"           # room 6, wave 2: the Engineer dies to a jab at 672
# name | room and plan | first:step:count | extra flags | crop L,T,R,B | still frame indices
clips=(
  "room2_getup|$R2|400:2:16||300,190,480,300|3,8"
  "room2_knockback_natural|$R2|790:4:10||300,190,480,300|3"
  "room2_crushed|$R2|790:2:24|--death-lab-engineer crushed|300,190,480,300|2,8"
  "room2_decapitated|$R2|790:2:24|--death-lab-engineer decapitated|300,190,480,300|2,6"
  "room2_exploded_preset4|$R2|790:2:24|--death-lab-engineer exploded|280,170,500,300|2,6"
  "room2_exploded_preset2|$R2|790:2:24|--death-lab-engineer exploded --gore-preset 2|280,170,500,300|2,6"
  "room2_slammed_stomped|$R2|790:2:24|--death-lab-engineer slammed_stomped|300,190,480,300|2,10"
  "room2_slammed_wall|$R2|790:2:24|--death-lab-engineer slammed_wall|300,190,480,300|2,10"
  "room2_slammed_flung|$R2|790:2:30|--death-lab-engineer slammed_flung|300,190,540,300|4,14"
  "room6_crushed|$R6|672:2:24|--death-lab-engineer crushed|340,200,540,300|2,8"
  "room6_decapitated|$R6|672:2:24|--death-lab-engineer decapitated|340,200,540,300|2,6"
  "room6_slammed_stomped|$R6|672:2:24|--death-lab-engineer slammed_stomped|340,200,540,300|2,10"
  "room6_slammed_flung|$R6|672:2:30|--death-lab-engineer slammed_flung|340,200,560,300|4,14"
)

capture_room() { # name room plan seq dir [flags...]
  local room="$2" plan="$3" seq="$4" dir="$5"
  shift 5
  bash scripts/room_capture.sh "$room" "$plan" "$seq" "$dir" "$@" | tail -n 1
}

for spec in "${clips[@]}"; do
  IFS='|' read -r name rp seq flags crop stills <<<"$spec"
  read -r room plan <<<"$rp"
  # shellcheck disable=SC2086
  l1="$(capture_room "$name" "$room" "$plan" "$seq" "$caps/$name-1" $flags)"
  # shellcheck disable=SC2086
  l2="$(capture_room "$name" "$room" "$plan" "$seq" "$caps/$name-2" $flags)"
  same=yes
  for f in "$caps/$name-1"/frame_*.png; do
    cmp -s "$f" "$caps/$name-2/$(basename "$f")" || same=no
  done
  hash1="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' <<<"$l1")"
  echo "ENG02_CAPTURE clip=$name room=$room plan=$plan seq=$seq flags=\"${flags:-none}\" window_hash=$hash1 $(sed -E 's/.*(headless_hash=[0-9a-f]+).*(match=[a-z]+).*/\1 \2/' <<<"$l1") generations_identical=$same" | tee -a "$lines"
  [[ "$l1" == *"match=yes"* && "$l2" == *"match=yes"* && "$same" == yes ]] || { echo "capture $name failed" >&2; exit 1; }
  step="$(sed -E 's/^[0-9]+:([0-9]+):.*/\1/' <<<"$seq")"
  still_args=()
  IFS=',' read -r -a idx <<<"$stills"
  for i in "${idx[@]}"; do still_args+=(--still "$i" "$out/${name}_f$(printf '%02d' "$i").png"); done
  python3 scripts/room_gif.py "$caps/$name-1" "$out/$name.gif" --step-ticks "$step" --crop "$crop" --zoom 3 "${still_args[@]}"
done

# The ram run: the CF02 arena fixture, the Engineer's ram passing under Carl's jump (ticks 34 to 56).
name=arena_ram_run
fixture=eng_jump_over_ram
for run in 1 2; do
  dir="$caps/$name-$run"
  mkdir -p "$dir"
  xvfb-run -a -s '-screen 0 1280x720x24' "$BIN" --scale 1 --party-fixture "$fixture" \
    --capture-sequence 34:2:12 --capture-dir "$dir" > "$dir/run.log" 2>&1 || true
  grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir/run.log" || { echo "$name window run $run failed" >&2; exit 1; }
done
"$BIN" --headless-floor01-smoke --party-fixture "$fixture" --capture-tick 56 > "$caps/$name-headless.log" 2>&1
state() { grep '^COMBAT_STATE ' "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
w1="$(state "$caps/$name-1/run.log")" w2="$(state "$caps/$name-2/run.log")" hh="$(state "$caps/$name-headless.log")"
same=yes
for f in "$caps/$name-1"/frame_*.png; do cmp -s "$f" "$caps/$name-2/$(basename "$f")" || same=no; done
match=no
[[ -n "$w1" && "$w1" == "$w2" && "$w1" == "$hh" ]] && match=yes
echo "ENG02_CAPTURE clip=$name fixture=$fixture seq=34:2:12 window_hash=$w1 headless_hash=$hh match=$match generations_identical=$same" | tee -a "$lines"
[[ "$match" == yes && "$same" == yes ]] || { echo "capture $name failed" >&2; exit 1; }
python3 scripts/room_gif.py "$caps/$name-1" "$out/$name.gif" --step-ticks 2 --crop 180,180,400,300 --zoom 3 \
  --still 3 "$out/${name}_f03.png" --still 5 "$out/${name}_f05.png" --still 7 "$out/${name}_f07.png"
echo "ENG02_EVIDENCE_DONE out=$out"
