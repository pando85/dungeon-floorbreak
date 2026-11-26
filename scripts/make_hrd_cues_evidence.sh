#!/usr/bin/env bash
# HRD-CUES-01: the window evidence of the Hoarder room's two cues (docs/02-art/hrd-cues-01-spec.md section 10): the pre-wake
# stir of the lair mound (S1 from a lead hero x of 240, S2 after the engage crossing) and the J-B post's "not yet" (`shored`
# while she is not out) and its 8-tick `give` beat.
#
# The cues' ages are render-local (the presenter's memo anchors them on the first tick it sees a level), so a single-tick
# capture would show every age at 0. Each cue is therefore captured as a ONE-TICK-PER-FRAME SEQUENCE that starts just before
# the cue is active, with scripts/room_capture.sh, and the stills are picked from it by tick:
#   s1     ticks 41..70   Carl reaches x 240 on tick 42 (S1 age 0), 3 px per tick
#   s2     ticks 107..116 Carl crosses x 432 on tick 108 (S2 age 0), the Reveal starts on tick 116
#   give   ticks 205..218 the last Reveal tick, she is out on 206 (give age 0..7 = 206..213), the machine frame from 214
#   shored tick 159       a Reveal tick (the shored post has no age)
# The pilot is `hoarder:carl_solo:0` (the room's solo pilot); the floor run is the same plan with `--f01-run`. Every sequence is
# captured TWICE per plan and size (640x360 at --scale 1, 1920x1080 at --scale 3): the two passes must be byte-identical frame
# by frame, the sequence's final window hash must equal the headless hash (room_capture.sh), and each kept still's own tick is
# also checked: the window's COMBAT_STATE hash of that tick must equal a headless run to that tick.
# Output: the stills, strips and GIFs, capture_lines.txt (one HRDCUE_STILL line per still and size) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and Python with Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/hrd-cues-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/hrd-cues-01}"
caps="${CAPTURES:-artifacts/hrd-cues}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_hoarder_mound.png && -f generated/art/floor01_hoarder_post_b.png ]] || { echo "the floor01_hoarder_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
pilot=hoarder:carl_solo:0
# sequence:first:count
seqs=(s1:41:30 s2:107:10 give:205:14 shored:159:1)
# name|sequence|tick|plans (d = direct plan, f = also the floor run)|what the still is
stills=(
  "stir_s1_heave|s1|42|df|S1 age 0: heave frame 3, no motes yet"
  "stir_s1_slump|s1|48|d|S1 age 6: slump frame 5, first mote"
  "stir_s1_motes|s1|54|d|S1 age 12: resting frame 0, three motes sliding"
  "stir_s1_rest|s1|66|d|S1 age 24: resting frame 0, two motes left"
  "stir_s2_heave|s2|111|df|S2 age 3: heave_hi frame 4, one mote"
  "stir_s2_late|s2|115|d|S2 age 7 (last Dormant tick): heave_hi frame 4, two motes"
  "shored|shored|159|df|a Reveal tick: post frame 3 (shored), no age"
  "shored_last|give|205|d|the last Reveal tick: shored"
  "give_0|give|206|df|out age 0: post frame 4 (give), 4 give motes with the 12-mote burst"
  "give_2|give|208|d|out age 2: give, burst and give motes"
  "give_7|give|213|d|out age 7: the last give tick, 4 give motes"
  "intact_after|give|216|df|out age 10: the machine's intact frame, no motes"
)
sizes=(640x360:1 1920x1080:3)
plans=(direct:"" floor:"--f01-run")
lines="$caps/capture_lines.txt"
: > "$lines"
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    res="${size%%:*}" scale="${size##*:}"
    for plan in "${plans[@]}"; do
      pname="${plan%%:*}" pflags="${plan#*:}"
      for seq in "${seqs[@]}"; do
        IFS=: read -r sname first count <<<"$seq"
        dir="$caps/pass$pass-$res-$pname-$sname"
        for attempt in 1 2 3; do
          # shellcheck disable=SC2086
          bash scripts/room_capture.sh hoarder "$pilot" "$first:1:$count" "$dir" $pflags --scale "$scale" > "$dir.txt" 2> "$dir.err" && break
          [[ "$attempt" -lt 3 ]] || { echo "capture $sname/$pname at $res failed 3 times: see $dir/run.log" >&2; exit 1; }
        done
      done
    done
  done
done
rm -f "$out"/*.png "$out"/*.gif "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
frame_of() { find "$1" -name "frame_*_tick_$(printf '%05d' "$2").png"; }
# the sequences: two passes byte-identical frame by frame, and the final hashes match (room_capture.sh checked it too)
for size in "${sizes[@]}"; do
  res="${size%%:*}"
  for plan in "${plans[@]}"; do
    pname="${plan%%:*}"
    for seq in "${seqs[@]}"; do
      IFS=: read -r sname first count <<<"$seq"
      a="$caps/pass1-$res-$pname-$sname" b="$caps/pass2-$res-$pname-$sname"
      same=yes
      for f in "$a"/frame_*.png; do cmp -s "$f" "$b/$(basename "$f")" || same=no; done
      [[ "$same" == yes ]] || fail=1
      grep -q 'match=yes' "$a.txt" || fail=1
      echo "HRDCUE_SEQUENCE plan=$pname size=$res seq=$sname first_tick=$first count=$count passes_byte_identical=$same $(sed -E 's/.* (window_hash=[0-9a-f]+ headless_hash=[0-9a-f]+).* (match=[a-z]+)$/\1 \2/' "$a.txt")" >> "$lines"
    done
  done
done
# headless hash at each kept tick, per plan (the window's COMBAT_STATE of that tick must equal it)
headless_at() { # plan flags tick
  # shellcheck disable=SC2086
  "$BIN" --headless-floor01-smoke --scene f01_late --f01-room hoarder --room-pilot "$pilot" $2 --capture-tick "$3" 2>&1 \
    | grep '^COMBAT_STATE ' | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'
}
for still in "${stills[@]}"; do
  IFS='|' read -r name sname tick which note <<<"$still"
  for plan in "${plans[@]}"; do
    pname="${plan%%:*}" pflags="${plan#*:}"
    [[ "$which" == *"${pname:0:1}"* ]] || continue
    headless="$(headless_at "$pname" "$pflags" "$tick")"
    for size in "${sizes[@]}"; do
      res="${size%%:*}"
      a="$caps/pass1-$res-$pname-$sname"
      frame="$(frame_of "$a" "$tick")"
      [[ -n "$frame" ]] || { echo "no frame for tick $tick in $a" >&2; exit 1; }
      [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
      window="$(grep "^COMBAT_STATE .* tick=$tick " "$a/run.log" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/')"
      match=no
      [[ -n "$window" && "$window" == "$headless" ]] && match=yes
      [[ "$match" == yes ]] || fail=1
      same=no
      cmp -s "$frame" "$(frame_of "$caps/pass2-$res-$pname-$sname" "$tick")" && same=yes
      [[ "$same" == yes ]] || fail=1
      stem="$name"
      [[ "$pname" == floor ]] && stem="floor_$name"
      cp "$frame" "$out/${stem}_$res.png"
      echo "HRDCUE_STILL name=$stem plan=$pname tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$frame" | cut -d' ' -f1) note=\"$note\"" >> "$lines"
    done
  done
done
python3 scripts/hrd_cues_strip.py "$caps/pass1-1920x1080-direct-s1" "$caps/pass1-1920x1080-direct-s2" "$caps/pass1-1920x1080-direct-give" "$out"
cp "$lines" "$out/capture_lines.txt"
(cd "$out" && sha256sum ./*.png ./*.gif ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
