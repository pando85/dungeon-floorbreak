#!/usr/bin/env bash
# DR-A: the screenshot evidence of room 3 (the Dozer revenge, "The Machine Remembers") drawn with the approved DOZ-01
# rig and the labelled stand-ins of the rear gate and the buffer stop (until DOZ-02, DZ-AD13).
#
# The aligned room pilot `dozer:close_call` (src/room_pilots.rs `dozer_escape`: hold at x 480 until the wheel is 64 px
# away, then run; DRA-AD5: Donut is the live party's autonomous `cf02_free` partner, as in a solo player's run) plays room 3 and scripts/room_capture.sh renders the real Bevy window under xvfb at the ticks below, at
# 640x360 (`--scale 1`) and at 640x400 (`--deck --scale 1`). Every capture is run TWICE (two full passes) and the PNGs
# of the two passes must be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals
# the headless run's (the presentation is read-only: it changes no sim hash). A short GIF of the chase (640x360, every
# 6th tick) is made from one more capture sequence, and one PLAYTRACE session of the same pilot is recorded in the real
# window and verified strict. Output: the PNGs, the GIF, capture_lines.txt (one DOZER_STILL line per still and size,
# one DOZER_GIF line), playtrace_session.txt and SHA256SUMS.
#
# DR-B, the reversal capture: the aligned room pilot `dozer:brace` (src/room_pilots.rs `dozer_brace`: Carl turns back to
# the brace at x 832, jabs it twice so the stop frame is released in the middle of the helper's useful window, holds
# while it lands on the machine, and runs once it is stunned) is captured the same way (stills `brace_*`, twice, both
# sizes, hash-checked) with a reversal GIF; the frame, the brace, its shadow and the remnant are labelled stand-ins. Perf is scripts/room_perf.sh (perf.txt, run on its own).
#
# DR-D (the #289 review finding 4 and INT DOZER-ROOM): two more aligned pilots are captured the same way. `dozer:retry`
# (src/room_pilots.rs `dozer_retry`: Carl stands at x 480 until the wheel takes him, respawns at the checkpoint, walks
# into the trigger, waits while the gate rumbles and runs once the machine is out) gives the stills `retry_*` and a
# retry GIF; `dozer:crew` (`dozer_crew`: the brace reversal, then the CF02 fight against the two dismounted riders and
# the exit) gives `crew_*` and a crew GIF. These stills also show the DR-D effects: the DOZ-01 metal hit-spark of a
# non-blast clang on the hull (a melee hit or a Magic Missile) and the ENV-02 splinter burst on the brace (`brace_release`) and in the rear gate.
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
# chase bands catch_up, cruise from 132, mercy from 237 (the close call: the riders yell), cruise from 297, the brake
# on 473 at the buffer stop and the escape (strain) on 498, the tick the first hero reaches the exit under the lintel.
stills=(trigger:30 chase:200 close_call:242 run:306 brake:476 exit:498)
brace=dozer:brace
# DR-B, the brace pilot's timeline (its headless COMBAT_EVENT lines): emerged on 111, Carl's jabs hit the brace on 303
# and 315 (the release, R 204), the frame falls from 344 and lands on 352 on the wheel front at x 728 (impact_stunned,
# by player 0), the machine is disabled on 400, and the room completes on 490.
brace_stills=(brace_release:318 brace_fall:348 brace_stun:353 brace_disabled:404 brace_run:440)
retry=dozer:retry
# DR-D, the retry pilot's timeline: the chase as in close_call until the wheel takes Carl at x 480 on 273 (one blow,
# hero_defeated), the checkpoint respawn on 322 (the world tick runs on: no restart), the rumble on 343 (by the trigger),
# the machine out of the gate on 433 (the gate's splinter bursts on 433..452), and the exit on 756.
retry_stills=(retry_caught:270 retry_down:300 retry_respawn:330 retry_gate:438 retry_escape:520)
crew=dozer:crew
# DR-D, the crew pilot's timeline: the brace reversal as in dozer:brace (released on 315, stunned on 352, disabled on
# 400), the riders dismount and fight; Carl's haymaker puts rider B down on 463, his jab puts rider A down on 515 and
# clangs on the disabled hull (the hit-spark, 515..520), and the exit on 693.
crew_stills=(crew_dismount:410 crew_fight:452 crew_clang:516 crew_exit:600)
all_stills=()
for still in "${stills[@]}"; do all_stills+=("$plan:$still"); done
for still in "${brace_stills[@]}"; do all_stills+=("$brace:$still"); done
for still in "${retry_stills[@]}"; do all_stills+=("$retry:$still"); done
for still in "${crew_stills[@]}"; do all_stills+=("$crew:$still"); done
sizes=(640x360:--scale:1 640x400:--deck:--scale:1)
lines="$caps/capture_lines.txt"
: > "$lines"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}"
    IFS=: read -r -a flags <<<"${size#*:}"
    for still in "${all_stills[@]}"; do
      IFS=: read -r _ splan name tick <<<"$still"
      splan="dozer:$splan"
      dir="$caps/pass$pass-$res-$name"
      # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the machine, not the
      # run, so a capture is retried up to three times. A hash mismatch is not retried away: it fails the checks below.
      for attempt in 1 2 3; do
        bash scripts/room_capture.sh dozer "$splan" "$tick:1:1" "$dir" "${flags[@]}" > "$dir.txt" 2> "$dir.err" && break
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
  for still in "${all_stills[@]}"; do
    IFS=: read -r _ splan name tick <<<"$still"
    splan="dozer:$splan"
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
    partner="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$caps/pass1-$res-$name/run.log" | tr ' ' '_')"
    [[ "$partner" == cf02_free ]] || fail=1
    echo "DOZER_STILL name=$name plan=$splan tick=$tick size=$res partner=$partner window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
# The chase GIF: ticks 174..498 every 6th tick, 640x360, 10 frames per second (0.6x game speed).
gif_dir="$caps/gif"
bash scripts/room_capture.sh dozer "$plan" "174:6:55" "$gif_dir" --scale 1 > "$gif_dir.txt" 2> "$gif_dir.err" || fail=1
ffmpeg -loglevel error -y -framerate 10 -pattern_type glob -i "$gif_dir/frame_*.png" \
  -vf "split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none" \
  "$out/chase_640x360.gif"
gif_window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$gif_dir.txt")"
gif_headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$gif_dir.txt")"
gif_match=no
gif_partner="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$gif_dir/run.log" | tr ' ' '_')"
[[ "$gif_partner" == cf02_free ]] || fail=1
[[ -n "$gif_window" && "$gif_window" == "$gif_headless" ]] && gif_match=yes
[[ "$gif_match" == yes ]] || fail=1
echo "DOZER_GIF name=chase plan=$plan ticks=174..498 step=6 frames=$(find "$gif_dir" -name 'frame_*.png' | wc -l) size=640x360 partner=$gif_partner window_hash=$gif_window headless_hash=$gif_headless hash_match=$gif_match" >> "$out/capture_lines.txt"
# DR-B, the reversal GIF: ticks 297..489 every 4th tick (the second jab, the warning, the fall, the stun, the disable and
# the run), 640x360, 10 frames per second (0.4x game speed).
rev_dir="$caps/gif-reversal"
bash scripts/room_capture.sh dozer "$brace" "297:4:49" "$rev_dir" --scale 1 > "$rev_dir.txt" 2> "$rev_dir.err" || fail=1
ffmpeg -loglevel error -y -framerate 10 -pattern_type glob -i "$rev_dir/frame_*.png" \
  -vf "split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none" \
  "$out/reversal_640x360.gif"
rev_window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$rev_dir.txt")"
rev_headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$rev_dir.txt")"
rev_partner="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$rev_dir/run.log" | tr ' ' '_')"
[[ "$rev_partner" == cf02_free ]] || fail=1
rev_match=no
[[ -n "$rev_window" && "$rev_window" == "$rev_headless" ]] && rev_match=yes
[[ "$rev_match" == yes ]] || fail=1
echo "DOZER_GIF name=reversal plan=$brace ticks=297..489 step=4 frames=$(find "$rev_dir" -name 'frame_*.png' | wc -l) size=640x360 partner=$rev_partner window_hash=$rev_window headless_hash=$rev_headless hash_match=$rev_match" >> "$out/capture_lines.txt"
# DR-D, the retry GIF (ticks 246..780 every 6th tick: caught, down, respawn, rumble, out of the gate, escape) and the crew
# GIF (ticks 396..692 every 4th tick: disabled, the dismounted riders' fight, the clang, the exit), 640x360, 10 fps.
dr_gif() {
  local name="$1" gplan="$2" spec="$3" ticks="$4" dir="$caps/gif-$1"
  bash scripts/room_capture.sh dozer "$gplan" "$spec" "$dir" --scale 1 > "$dir.txt" 2> "$dir.err" || fail=1
  ffmpeg -loglevel error -y -framerate 10 -pattern_type glob -i "$dir/frame_*.png" \
    -vf "split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=none" \
    "$out/${name}_640x360.gif"
  local w h p m=no step
  w="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
  h="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$dir.txt")"
  p="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$dir/run.log" | tr ' ' '_')"
  [[ "$p" == cf02_free ]] || fail=1
  [[ -n "$w" && "$w" == "$h" ]] && m=yes
  [[ "$m" == yes ]] || fail=1
  IFS=: read -r _ step _ <<<"$spec"
  echo "DOZER_GIF name=$name plan=$gplan ticks=$ticks step=$step frames=$(find "$dir" -name 'frame_*.png' | wc -l) size=640x360 partner=$p window_hash=$w headless_hash=$h hash_match=$m" >> "$out/capture_lines.txt"
}
dr_gif retry "$retry" "246:6:90" "246..780"
dr_gif crew "$crew" "396:4:75" "396..692"
# One recorded PLAYTRACE session of room 3 (since DR-B the reversal pilot `dozer:brace`, real window, ticks 0 to 520: the
# frame lands on the machine on 352 and the room completes on 490), verified strict (scope=full) with the replay rebuild
# of the late-Floor-1 source.
pt="${PT:-./target/debug/playtrace}"
sdir="$caps/session"; slog="$caps/session.log"; mkdir -p "$sdir"
xvfb-run -a -s '-screen 0 1280x720x24' "$BIN" --scale 1 --no-audio --scene f01_late --f01-room dozer --room-pilot "$brace" \
  --record --record-dir "$sdir" --exit-after-ticks 520 > "$slog" 2>&1 || true
sid="$(ls "$sdir")"
{
  echo "== room=dozer plan=$brace label=$brace exit_after_ticks=520"
  grep -E '^F01_LATE_ENTRY' "$slog" | sed -E 's/^(F01_LATE_ENTRY room=[a-z]+).*(loadout=[a-z]+)$/\1 \2/'
  grep -E '^COMPANION_CONFIG' "$slog"
  grep '^PLAYTRACE_SAVED ' "$slog" | sed -E 's/^(PLAYTRACE_SAVED) session=[0-9a-f]+ (.*)/\1 \2/'
  "$pt" verify "$sid" --dir "$sdir" --root . | grep -E '^PLAYTRACE_VERIFY(_CHECKS)? ' | sed -E 's/session=[0-9a-f]+ //'
  "$pt" inspect "$sid" --dir "$sdir" | grep -E '^  (schema|reset|ticks|episodes)' | sed -E 's/^  /inspect: /; s/ build .*//'
} > "$out/playtrace_session.txt"
grep -q 'result=PASS scope=full' "$out/playtrace_session.txt" || fail=1
# DRA-AD5: the partner is the live party's `cf02_free` companion, active (not the legacy follower).
grep -qx 'COMPANION_CONFIG: set=cf02_free' "$out/playtrace_session.txt" || { echo "the session's partner is not the active cf02_free companion" >&2; fail=1; }
(cd "$out" && sha256sum ./*.png ./*.gif ./*.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt" "$out/playtrace_session.txt"
exit "$fail"
