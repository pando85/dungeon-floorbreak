#!/usr/bin/env bash
# DOZ-02: the Bevy evidence of room 3's damage states (docs/02-art/doz-02-dozer-damage-spec.md): native captures of the
# real window under xvfb at 640x360 (`--scale 1`) and 640x400 (`--deck --scale 1`) for every state the aligned room
# pilots `dozer:brace`, `dozer:retry` and `dozer:crew` reach (src/room_pilots.rs; the cf02_free partner), and the
# reversal GIF (one frame per 60 Hz tick from the release to 60 ticks of `disabled`).
#
# Every still is captured TWICE (two full passes) and the PNGs of the two passes must be byte-identical; each capture
# also checks that the window's final COMBAT_STATE hash equals the headless run's (the presentation is read-only: it
# changes no simulation hash). Output: doz02_bevy_<name>_<size>.png, doz02_capture_lines.txt (one DOZ02_STILL line per
# still and size, one DOZ02_GIF line) and the reversal frames the Python script turns into the GIFs.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and ffmpeg.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   OUT=<evidence dir> (default docs/evidence/doz-02)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/doz-02}"
caps="${CAPTURES:-artifacts/doz02}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/dozer_hull_damage_h120.png && -f generated/art/prop_stop_frame.png ]] || { echo "generated/art/*.png are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# plan:name:tick. The timelines are the pilots' headless COMBAT_EVENT lines: `dozer:brace` jabs the brace on 303 and 315
# (the release), the frame lands on the front on 352 (impact_stunned, 48 ticks), the machine is disabled on 400, the
# riders dismount then; `dozer:retry` has the machine out of the gate on 433; `dozer:crew` is the brace pilot and the fight.
stills=(
  brace:gate_closed:10
  brace:gate_shake:60
  brace:brace_weakened:306
  brace:warning:325
  brace:fall:348
  brace:impact_pitch0:353
  brace:impact_pitch1:359
  brace:impact_tumble:370
  brace:impact_lying:392
  brace:disabled:410
  retry:gate_burst:438
  crew:crew_fight:452
  crew:exit_tunnel:660
  brace:disabled_machine:364
)
sizes=(640x360:--scale:1 640x400:--deck:--scale:1)
lines="$caps/capture_lines.txt"
: > "$lines"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}"
    IFS=: read -r -a flags <<<"${size#*:}"
    for still in "${stills[@]}"; do
      IFS=: read -r splan name tick <<<"$still"
      dir="$caps/pass$pass-$res-$name"
      # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the machine, not the
      # run, so a capture is retried up to three times. A hash mismatch is not retried away: it fails the checks below.
      for attempt in 1 2 3; do
        bash scripts/room_capture.sh dozer "dozer:$splan" "$tick:1:1" "$dir" "${flags[@]}" > "$dir.txt" 2> "$dir.err" && break
        [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
      done
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/doz02_bevy_*.png "$out"/doz02_capture_lines.txt
fail=0
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    IFS=: read -r splan name tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no
    cmp -s "$a" "$b" && same=yes
    [[ "$same" == yes ]] || fail=1
    window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$res-$name.txt")"
    match=no
    [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$match" == yes ]] || fail=1
    cp "$a" "$out/doz02_bevy_${name}_$res.png"
    partner="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$caps/pass1-$res-$name/run.log" | tr ' ' '_')"
    [[ "$partner" == cf02_free ]] || fail=1
    echo "DOZ02_STILL name=$name plan=dozer:$splan tick=$tick size=$res partner=$partner window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/doz02_capture_lines.txt"
  done
done
# The reversal frames: every tick from just before the release (the second jab, 315) to 60 ticks of `disabled` (400..460).
rev_dir="$caps/reversal"
bash scripts/room_capture.sh dozer dozer:brace "312:1:150" "$rev_dir" --scale 1 > "$rev_dir.txt" 2> "$rev_dir.err" || fail=1
rev_window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$rev_dir.txt")"
rev_headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$rev_dir.txt")"
rev_partner="$(sed -nE 's/^COMPANION_CONFIG: set=//p' "$rev_dir/run.log" | tr ' ' '_')"
[[ "$rev_partner" == cf02_free ]] || fail=1
rev_match=no
[[ -n "$rev_window" && "$rev_window" == "$rev_headless" ]] && rev_match=yes
[[ "$rev_match" == yes ]] || fail=1
echo "DOZ02_GIF name=reversal plan=dozer:brace ticks=312..461 step=1 frames=$(find "$rev_dir" -name 'frame_*.png' | wc -l) size=640x360 partner=$rev_partner window_hash=$rev_window headless_hash=$rev_headless hash_match=$rev_match" >> "$out/doz02_capture_lines.txt"
mkdir -p "$caps/reversal_frames"
cp "$rev_dir"/frame_*.png "$caps/reversal_frames/"
cat "$out/doz02_capture_lines.txt"
exit "$fail"
