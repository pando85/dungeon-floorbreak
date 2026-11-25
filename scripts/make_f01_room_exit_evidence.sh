#!/usr/bin/env bash
# ROOM-PORTALS-01: the in-scene evidence of the room exit door of rooms 2, 4, 4a, 4b and 6 (the drop grate down
# while the `defeat_roster` exit gate stands, raised once it opens). Spec: docs/02-art/floor01-room-exit-spec.md.
#
# The aligned room pilots play the rooms and scripts/room_capture.sh renders the real Bevy window under
# xvfb at the ticks below, at 1x (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`):
#   - room 6 (mastery, `mastery:carl_solo:0`): the grate down during the llama yard wave, then raised
#     after the clear and Carl walking into the open exit;
#   - room 4 (hazards, `hazards:carl_solo:0`): the raised grate as Carl reaches the exit (every aligned
#     room 4 route reaches the exit's view only after the clear; the closed frame of room 4 is the same
#     code path as room 6's and is held by `room_exit_art::tests`);
#   - room 2 (boiler, `boiler:carl_solo:direct:0`): the exit open (the Boiler has no exit gate);
#   - room 4b (ring_split): the exit open, from the room's scripted walk-right smoke (the Ring rooms have no room
#     pilot yet; room 4a's walk stops at its closed shutter, so its exit is never in view: the unit tests hold it).
# Every capture is run TWICE (two full passes) and the PNGs of the two passes must be byte-identical; each
# capture also checks that the window's final COMBAT_STATE hash equals the headless run's (the presentation
# is read-only). It also renders one game-speed GIF of the room 6 clear (the grate snaps up on the tick the
# last roster enemy falls). Output: the PNGs, the GIF, capture_lines.txt and SHA256SUMS in $OUT/scene.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh: generated/art/f01-dfx-exit.png), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/f01-room-exit)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-room-exit}/scene"
caps="${CAPTURES:-artifacts/room-exit}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/f01-dfx-exit.png ]] || { echo "generated/art/f01-dfx-exit.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:plan:name:tick. mastery:carl_solo:0 fights the llama yard wave in view of the exit (the grate down)
# until its roster is clear between ticks 660 and 690, then heads for the exit (the pilot's exit tick is 737).
stills=(
  mastery:mastery:carl_solo:0:room6_closed:600
  mastery:mastery:carl_solo:0:room6_open:700
  mastery:mastery:carl_solo:0:room6_walk_in:735
  hazards:hazards:carl_solo:0:room4_open:745
  boiler:boiler:carl_solo:direct:0:room2_open:830
)
sizes=(640x360:1 1920x1080:3)
capture() { # room plan seq dir [flags]
  local room="$1" plan="$2" seq="$3" dir="$4"
  shift 4
  # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the
  # machine, not the run, so a capture is retried up to three times. A hash mismatch is not retried
  # away: it fails the checks below.
  for attempt in 1 2 3; do
    bash scripts/room_capture.sh "$room" "$plan" "$seq" "$dir" "$@" > "$dir.txt" 2> "$dir.err" && return 0
    [[ "$attempt" -lt 3 ]] || { echo "capture $dir failed 3 times: see $dir/run.log" >&2; exit 1; }
  done
}
# A still entry: the room, the plan (it holds colons), the name and the tick.
split() {
  local entry="$1"
  room="${entry%%:*}"
  local rest="${entry#*:}"
  tick="${rest##*:}"
  rest="${rest%:*}"
  name="${rest##*:}"
  plan="${rest%:*}"
}
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    for still in "${stills[@]}"; do
      split "$still"
      dir="$caps/pass$pass-$res-$name"
      capture "$room" "$plan" "$tick:1:1" "$dir" --scale "$scale"
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      grep -q '^ROOM_EXIT drawn' "$dir/run.log" || { echo "the exit door was not drawn: $dir/run.log" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
# Room 4b: no room pilot, so the scripted walk-right smoke of the room scene (no `--room-pilot`), the window run and
# the headless run to the same tick, as scripts/room_capture.sh does (the walk reaches the exit on tick 312).
ring_tick=250
ring_state() { grep "^COMBAT_STATE " "$1" | grep " tick=$ring_tick " | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    dir="$caps/pass$pass-$res-room4b_open"
    rm -rf "$dir"; mkdir -p "$dir"
    flock -x "$lock" xvfb-run -a -s '-screen 0 1280x720x24' "$BIN" --scale "$scale" --scene f01_late --f01-room ring_split \
      --capture-sequence "$ring_tick:1:1" --capture-dir "$dir" > "$dir/run.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir/run.log" || { echo "ring_split window run failed: $dir/run.log" >&2; exit 1; }
    grep -q '^ROOM_EXIT drawn' "$dir/run.log" || { echo "the exit door was not drawn: $dir/run.log" >&2; exit 1; }
    "$BIN" --headless-floor01-smoke --scene f01_late --f01-room ring_split --capture-tick "$ring_tick" > "$dir/headless.log" 2>&1
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-room4b_open_$res.png"
    echo "$(ring_state "$dir/run.log") $(ring_state "$dir/headless.log")" > "$dir.hashes"
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    split "$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no
    cmp -s "$a" "$b" && same=yes
    [[ "$same" == yes ]] || fail=1
    window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    match=no
    [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$match" == yes ]] || fail=1
    cp "$a" "$out/${name}_$res.png"
    echo "ROOM_EXIT_STILL room=$room plan=$plan name=$name tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  a="$caps/pass1-room4b_open_$res.png" b="$caps/pass2-room4b_open_$res.png"
  same=no
  cmp -s "$a" "$b" && same=yes
  [[ "$same" == yes ]] || fail=1
  read -r window headless < "$caps/pass1-$res-room4b_open.hashes"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$match" == yes ]] || fail=1
  cp "$a" "$out/room4b_open_$res.png"
  echo "ROOM_EXIT_STILL room=ring_split plan=walk_right_smoke name=room4b_open tick=$ring_tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
# The room 6 clear at game speed: the last roster enemies fall, the grate snaps up, Carl walks in
# (ticks 630..738, every 3 ticks), generated twice and compared byte for byte.
for pass in 1 2; do
  dir="$caps/gif$pass-room6"
  capture mastery mastery:carl_solo:0 630:3:37 "$dir"
  python3 scripts/room_gif.py "$dir" "$caps/gif$pass-room6_clear_640x360.gif" --step-ticks 3
done
same=no
cmp -s "$caps/gif1-room6_clear_640x360.gif" "$caps/gif2-room6_clear_640x360.gif" && same=yes
[[ "$same" == yes ]] || fail=1
cp "$caps/gif1-room6_clear_640x360.gif" "$out/room6_clear_640x360.gif"
window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/gif1-room6.txt")"
headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/gif1-room6.txt")"
match=no
[[ -n "$window" && "$window" == "$headless" ]] && match=yes
[[ "$match" == yes ]] || fail=1
echo "ROOM_EXIT_GIF room=mastery plan=mastery:carl_solo:0 name=room6_clear ticks=630..738 step=3 window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same" >> "$out/capture_lines.txt"
(cd "$out" && sha256sum ./*.png ./*.gif ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
