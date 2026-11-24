#!/usr/bin/env bash
# Evidence for Donut's wired damage-reaction frames (ANIM-DONUT-02 melee frames 8 to 12) and the
# shared i-frame flicker (FA-Q38) in the Floor 1 scene: the real Bevy window under xvfb, at 1080p (--scale 3), on the two committed party
# fixtures that hurt and defeat her (and one Carl fixture for the i-frame cue). Ten ticks, each captured TWICE: the PNGs of the two passes must
# be byte-identical, and the window's final COMBAT_STATE hash must equal the headless run's (the
# presentation changes no sim hash). Each still is cropped around Donut (a pure crop of the scene
# pixels, no resampling); strip_x6.png lays the crops side by side, enlarged 2x with
# nearest-neighbour (6x the 640x360 canvas) and labelled.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/f01-donut-hurt)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/f01-donut-hurt}"
caps="${CAPTURES:-artifacts/donut-hurt}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/donut_h40_melee.png ]] || { echo "generated/art/donut_h40_melee.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:kind(party-fixture|fixture):fixture:tick:crop(WxH+X+Y on the 1920x1080 frame):label. The ticks are the state after the
# tick: the hurt fixture's blow lands on tick 64 (h0 on ticks 64 to 72, h1 on 73 to 85); the defeat
# fixture's lethal blow on tick 349 (the freeze on h0 to 356, the thrown frame 357 to 373, d0 374 to
# 379, d1 380 to the restart on 398).
stills=(
  "hurt_h0:party-fixture:donut_hurt_iframes:68:320x210+790+680:HURT h0 (t68)"
  "hurt_h1:party-fixture:donut_hurt_iframes:77:320x210+790+680:HURT h1 (t77)"
  "defeat_freeze_h0:party-fixture:donut_defeat_party:352:320x210+790+680:DEFEAT FREEZE h0 (t352)"
  "knockback_kb:party-fixture:donut_defeat_party:364:320x210+800+680:KNOCKBACK kb (t364)"
  "knockdown_d0:party-fixture:donut_defeat_party:375:320x210+830+680:KNOCKDOWN d0 (t375)"
  "knockdown_d1:party-fixture:donut_defeat_party:390:320x210+830+680:KNOCKDOWN d1 (t390)"
  "ifr_donut_dim:party-fixture:donut_hurt_iframes:96:320x210+790+680:DONUT I-FRAMES dim (t96)"
  "ifr_donut_bright:party-fixture:donut_hurt_iframes:100:320x210+790+680:DONUT I-FRAMES bright (t100)"
  "ifr_carl_dim:fixture:cp01_goblin_attacks:188:320x210+700+680:CARL I-FRAMES dim (t188)"
  "ifr_carl_bright:fixture:cp01_goblin_attacks:192:320x210+700+680:CARL I-FRAMES bright (t192)"
)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r name kind fixture tick crop label <<<"$still"
    dir="$caps/pass$pass-$name"
    mkdir -p "$dir"
    # A window can lose its GPU device when other jobs share the GPU: retry up to three times.
    for attempt in 1 2 3; do
      xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --scale 3 "--$kind" "$fixture" \
        --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir/run.log" 2>&1 || true
      grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir/run.log" && break
      [[ "$attempt" -lt 3 ]] || { echo "capture $name failed 3 times: see $dir/run.log" >&2; exit 1; }
    done
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == 1920x1080 ]] || { echo "bad size: $frame" >&2; exit 1; }
    "$bin" --headless-floor01-smoke "--$kind" "$fixture" --capture-tick "$tick" > "$dir/headless.log" 2>&1
    magick "$frame" -crop "$crop" +repage -strip -define png:exclude-chunks=date,time "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
labels=()
for still in "${stills[@]}"; do
  IFS=: read -r name kind fixture tick crop label <<<"$still"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no
  cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name/run.log")"
  headless="$(state "$caps/pass1-$name/headless.log")"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/$name.png"
  labels+=(-label "$label" "$out/$name.png")
  echo "DONUT_HURT_STILL name=$name fixture=$fixture tick=$tick window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
magick montage -define png:exclude-chunks=date,time -strip -font DejaVu-Sans -pointsize 18 -background '#1b1b1b' -fill white "${labels[@]}" \
  -filter point -resize 200% -tile 3x4 -geometry +6+6 "$out/strip_x6.png"
(cd "$out" && sha256sum ./*.png ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
