#!/usr/bin/env bash
# FA-Q23 (HRD-P) and HOARD-02: the screenshot evidence of the Hoarder room's presentation (the HOARD-02 boss atlas and
# the floor01_hoarder_* room kit, no stand-in label).
#
# The room pilot `hoarder:carl_solo:0` (src/hoarder_fixtures.rs `ClearPilot`) plays room 1 and
# scripts/room_capture.sh renders the real Bevy window under xvfb at the ticks below, at 1x
# (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`). Every capture is run TWICE
# (two full passes) and the PNGs of the two passes must be byte-identical; each capture also
# checks that the window's final COMBAT_STATE hash equals the headless run's (the presentation
# is read-only: it changes no sim hash). Output: the twelve PNGs, capture_lines.txt (one
# HOARDER_STILL line per still and size) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh: generated/art/hoard-02.png and floor01_hoarder_*.png), xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/f01-hoarder-presentation)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-hoarder-presentation}"
caps="${CAPTURES:-artifacts/hoarder-presentation}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/hoard-02.png && -f generated/art/floor01_hoarder_mound.png ]] || { echo "generated/art/hoard-02.png or the floor01_hoarder_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:tick: the dormant lair (sealed mound), the Reveal in thirds (frames 26, 27, 28: shadow tint, lip in front), the last
# Reveal tick, the first tick she can be hit (full value, the opened mound, the dust burst), the swipe's rear-up tell, its
# impact, the free recovery, and the defeat (down frame 15, the door open). Ticks of the pilot hoarder:carl_solo:0: Reveal
# 116..205, SwipeTell from 206, SwipeSmear 240, SwipeImpact 243, SwipeRecover 249 (HRD-B/HRD-C, #260, moved them 4 later). The room pilot kills her before she
# shoves, so the shove, lunge, jam, pinned and get-up have offline composites instead (docs/evidence/hoard-02).
stills=(dormant:100 reveal1:129 reveal2:159 reveal3:189 reveal_last:205 out:206 out_late:210 tell:222 impact:245 recover:266 defeated:1300)
sizes=(640x360:1 1920x1080:3)
lines="$caps/capture_lines.txt"
: > "$lines"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    for still in "${stills[@]}"; do
      name="${still%%:*}" tick="${still##*:}"
      dir="$caps/pass$pass-$res-$name"
      # A window can lose its GPU device when other jobs share the GPU ("Device(Lost)"): that is the
      # machine, not the run, so a capture is retried up to three times. A hash mismatch is not retried
      # away: it fails the checks below.
      for attempt in 1 2 3; do
        bash scripts/room_capture.sh hoarder hoarder:carl_solo:0 "$tick:1:1" "$dir" --scale "$scale" > "$dir.txt" 2> "$dir.err" && break
        [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
      done
      frame="$(find "$dir" -name 'frame_*.png')"
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for still in "${stills[@]}"; do
    name="${still%%:*}" tick="${still##*:}"
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
    echo "HOARDER_STILL name=$name tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
(cd "$out" && sha256sum ./*.png ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
