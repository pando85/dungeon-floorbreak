#!/usr/bin/env bash
# DR-A: the screenshot evidence of room 3 (the Dozer revenge, "The Machine Remembers") drawn with the approved DOZ-01
# rig and the labelled stand-ins of the rear gate and the buffer stop (until DOZ-02, DZ-AD13).
#
# The room pilot `dozer:close_call` (src/room_pilots.rs `dozer_escape`: hold at x 480 until the wheel is 64 px away,
# then run) plays room 3 and scripts/room_capture.sh renders the real Bevy window under xvfb at the ticks below, at
# 640x360 (`--scale 1`) and at 640x400 (`--deck --scale 1`). Every capture is run TWICE (two full passes) and the PNGs
# of the two passes must be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals
# the headless run's (the presentation is read-only: it changes no sim hash). A short GIF of the chase (640x360, every
# 6th tick) is made from one more capture sequence, and one PLAYTRACE session of the same pilot is recorded in the real
# window and verified strict. Output: the PNGs, the GIF, capture_lines.txt (one DOZER_STILL line per still and size,
# one DOZER_GIF line), playtrace_session.txt and SHA256SUMS. Perf is scripts/room_perf.sh (perf.txt, run on its own).
#
# Needs the debug binaries (cargo build --locked --bin floor01_scene --bin playtrace), the exported atlases (bash
# scripts/export_assets.sh: generated/art/dozer_*.png), xvfb-run, flock, ImageMagick and ffmpeg.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   PT=<playtrace> (default ./target/debug/playtrace)
#   OUT=<evidence dir> (default docs/evidence/f01-dozer-revenge)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-dozer-revenge}"
caps="${CAPTURES:-artifacts/dozer-revenge}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/dozer_hull_h120.png && -f generated/art/dozer_wheel_h120.png ]] || { echo "generated/art/dozer_*.png are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
plan=dozer:close_call
# name:tick. Timeline of the pilot (the headless run's COMBAT_EVENT lines): the trigger and the rumble on tick 21 (the
# gate shakes, 90 ticks), the machine emerges on 111 (off screen: the camera follows Carl at the hold column), the
# chase bands catch_up, cruise from 170, mercy from 231 (the close call: the riders yell), cruise from 291, the brake
# on 467 at the buffer stop and the escape (strain) on 492, the tick Carl reaches the exit under the lintel.
stills=(trigger:30 chase:200 close_call:236 run:300 brake:470 exit:492)
sizes=(640x360:--scale:1 640x400:--deck:--scale:1)
lines="$caps/capture_lines.txt"
: > "$lines"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}"
    IFS=: read -r -a flags <<<"${size#*:}"
    for still in "${stills[@]}"; do
      IFS=: read -r name tick <<<"$still"
      dir="$caps/pass$pass-$res-$name"
      # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the machine, not the
      # run, so a capture is retried up to three times. A hash mismatch is not retried away: it fails the checks below.
      for attempt in 1 2 3; do
        bash scripts/room_capture.sh dozer "$plan" "$tick:1:1" "$dir" "${flags[@]}" > "$dir.txt" 2> "$dir.err" && break
        [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
      done
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/playtrace_session.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    IFS=: read -r name tick <<<"$still"
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
    echo "DOZER_STILL name=$name plan=$plan tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
# The chase GIF: ticks 168..492 every 6th tick, 640x360, 10 frames per second (0.6x game speed).
gif_dir="$caps/gif"
bash scripts/room_capture.sh dozer "$plan" "168:6:55" "$gif_dir" --scale 1 > "$gif_dir.txt" 2> "$gif_dir.err" || fail=1
ffmpeg -loglevel error -y -framerate 10 -pattern_type glob -i "$gif_dir/frame_*.png" \
  -vf "split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none" \
  "$out/chase_640x360.gif"
gif_window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$gif_dir.txt")"
gif_headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$gif_dir.txt")"
gif_match=no
[[ -n "$gif_window" && "$gif_window" == "$gif_headless" ]] && gif_match=yes
[[ "$gif_match" == yes ]] || fail=1
echo "DOZER_GIF name=chase plan=$plan ticks=168..492 step=6 frames=$(find "$gif_dir" -name 'frame_*.png' | wc -l) size=640x360 window_hash=$gif_window headless_hash=$gif_headless hash_match=$gif_match" >> "$out/capture_lines.txt"
# One recorded PLAYTRACE session of room 3 (the same pilot, real window, ticks 0 to 520: the exit is reached on tick 492),
# verified strict (scope=full) with the replay rebuild of the late-Floor-1 source.
pt="${PT:-./target/debug/playtrace}"
sdir="$caps/session"; slog="$caps/session.log"; mkdir -p "$sdir"
xvfb-run -a -s '-screen 0 1280x720x24' "$BIN" --scale 1 --no-audio --scene f01_late --f01-room dozer --room-pilot "$plan" \
  --record --record-dir "$sdir" --exit-after-ticks 520 > "$slog" 2>&1 || true
sid="$(ls "$sdir")"
{
  echo "== room=dozer plan=$plan label=$plan exit_after_ticks=520"
  grep -E '^F01_LATE_ENTRY' "$slog" | sed -E 's/^(F01_LATE_ENTRY room=[a-z]+).*(loadout=[a-z]+)$/\1 \2/'
  grep '^PLAYTRACE_SAVED ' "$slog" | sed -E 's/^(PLAYTRACE_SAVED) session=[0-9a-f]+ (.*)/\1 \2/'
  "$pt" verify "$sid" --dir "$sdir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
  "$pt" inspect "$sid" --dir "$sdir" | grep -E '^  (schema|reset|ticks|episodes)' | sed -E 's/^  /inspect: /; s/ build .*//'
} > "$out/playtrace_session.txt"
grep -q 'result=PASS scope=full' "$out/playtrace_session.txt" || fail=1
(cd "$out" && sha256sum ./*.png ./*.gif ./*.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt" "$out/playtrace_session.txt"
exit "$fail"
