#!/usr/bin/env bash
# CF02-I evidence: real Bevy 640x360 (1x) GIFs of the CF02 acting frames.
#
#   cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
#   bash scripts/make_cf02_i_evidence.sh [out_dir] [clip-name ...]
#
# Each clip replays a committed `cf02_*` fixture in the real render loop
# (lighting on, gore preset 4, CF02 rules on) and captures the window at every
# STEP-th tick with --capture-tick, crops a window around the actors (the
# camera keeps Carl near the centre), and joins the frames into a GIF that plays
# in real time (STEP ticks = STEP/60 s per frame, at native pixels: no scaling).
# It also writes the COMBAT_ANIM_STATE line of every frame, so each image is tied
# to the frame the animators chose. Needs xvfb-run, ImageMagick and a debug build.
# Presentation evidence only; not reproduced bit for bit by CI (driver dependent).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export FLOORBREAK_RECORD=off
out="${1:-docs/evidence/cf02/cf02-i}"
shift || true
bin=./target/debug/floor01_scene
# name|fixture|first tick|last tick|ticks per frame|crop (WxH+X+Y of the 640x360 view, optional)
clips=(
  "carl_stomp|cf02_stomp_standing|4|40|2"
  "carl_follow_kick|cf02_confirm_kick|14|52|2"
  "carl_air_kick|cf02_air_kick_jump_in|0|32|2|192x190+200+110"
  "carl_dive_and_heavy_landing|cf02_dive_whiff_recovery|14|52|2|192x190+200+110"
  "carl_bailout|cf02_bailout_escapes_swing|26|66|2|192x190+200+110"
  "carl_flinch|cf02_llama_bolt_stops|86|106|2"
  "donut_startle|cf02_wall_slam_kill|30|52|2"
  "donut_smug|cf02_assist_pounce_staggered|56|86|2|288x108+176+190"
  "goblin_hesitate|cf02_morale_witness_hesitate|14|46|2|288x108+176+190"
  "goblin_panic|cf02_morale_spectacle_panic|38|72|2|320x150+160+150"
  "goblin_taunt_punished|cf02_showoff_taunt_punish|6|52|2"
  "slammed_stomped|cf02_kick_then_stomp|66|98|2|288x108+176+190"
  "slammed_wall|cf02_wall_slam_kill|24|58|2"
  "slammed_flung|cf02_lethal_kick_far|60|100|2|320x108+160+190"
  "arena_carl_frustration|cf02_block_hurts_carl|62|98|2|288x108+176+190"
  "arena_goblin_panic|cf02_arena_direct|116|156|2|320x150+160+150"
  "arena_goblin_taunt|cf02_arena_direct|366|410|2|288x108+176+190"
  "arena_donut_smug|cf02_arena_direct|378|406|2|288x108+176+190"
  "donut_startle_clear|cf02_showoff_taunt_punish|106|124|2|288x108+176+190"
  "arena_goblin_hesitate|cf02_arena_coop|116|142|2|288x108+176+190"
  "arena_slammed_flung|cf02_goblin_self_own|88|120|2|320x108+160+190"
  # CF02-FU-I FU1 (F01-FU-B10): her smug in a clear shot (no one within 96 px).
  "donut_smug_clear|cf02_free_smug_clear|156|190|2|288x108+112+190"
)
extra_clips="${CF02_I_EXTRA_CLIPS:-}"
if [[ -n "$extra_clips" ]]; then
  IFS=';' read -r -a more <<<"$extra_clips"
  clips+=("${more[@]}")
fi
mkdir -p "$out"

capture() { # fixture tick png
  local log pid
  log="$(mktemp)"
  "$bin" --scale 1 --party-fixture "$1" --capture-tick "$2" >"$log" 2>&1 &
  pid=$!
  for _ in $(seq 1 120); do
    grep -qE 'FLOOR1_TEXTURES_READY|FLOOR1_ASSET_FAILURE' "$log" && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  if ! grep -q FLOOR1_TEXTURES_READY "$log"; then
    cat "$log" >&2
    kill "$pid" 2>/dev/null || true
    return 1
  fi
  sleep 1.5
  import -window root -crop 640x360+0+0 +repage "$3"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  if grep -qE 'FLOOR1_ASSET_FAILURE|FLOOR1_LIGHTING_WARNING' "$log"; then
    cat "$log" >&2
    return 1
  fi
  grep '^COMBAT_ANIM_STATE' "$log" | tail -n 1 >>"${3%/*}/states.txt"
  rm -f "$log"
}

render_clip() {
  local name="$1" fixture="$2" first="$3" last="$4" step="$5" crop="$6"
  local dir frames=()
  dir="$(mktemp -d)"
  : >"$dir/states.txt"
  for tick in $(seq "$first" "$step" "$last"); do
    local png
    png="$dir/f_$(printf '%05d' "$tick").png"
    # A dark first frame happens while the lighting warms up: retry it.
    for attempt in 1 2 3; do
      capture "$fixture" "$tick" "$png"
      mean="$(magick "$png" -colorspace Gray -format '%[fx:mean]' info:)"
      if awk -v m="$mean" 'BEGIN { exit !(m > 0.05) }'; then break; fi
      sed -i '$d' "$dir/states.txt"
      [[ "$attempt" -lt 3 ]] || { echo "$name stayed dark at tick $tick" >&2; return 1; }
    done
    magick "$png" -crop "$crop" +repage "$png"
    frames+=("$png")
  done
  # GIF delay is in 1/100 s: STEP ticks at 60 Hz, rounded (2 ticks = 3).
  local delay=$(( (step * 100 + 30) / 60 ))
  magick -delay "$delay" "${frames[@]}" -delay 60 "${frames[-1]}" -loop 0 -layers Optimize "$out/${name}.gif"
  # A 1x strip (every frame side by side, two rows) for a still review.
  magick montage "${frames[@]}" -tile 6x -geometry +1+1 -background '#101010' "$out/${name}_strip.png"
  { echo "# $name: fixture $fixture, ticks $first..$last step $step"; cat "$dir/states.txt"; } >"$out/${name}_states.txt"
  rm -rf "$dir"
  echo "CF02_I_CLIP_DONE: $name"
}

# Worker: render one clip on the display this process runs under.
if [[ "${CF02_I_WORKER:-}" == 1 ]]; then
  IFS='|' read -r name fixture first last step crop <<<"$CF02_I_SPEC"
  render_clip "$name" "$fixture" "$first" "$last" "$step" "${crop:-192x108+200+190}"
  exit 0
fi
# Driver: one private X display per clip (a window of another clip must never be
# in the screenshot), CF02_I_JOBS clips at a time (the machine is shared).
want=("$@")
pids=()
for spec in "${clips[@]}"; do
  IFS='|' read -r name _ <<<"$spec"
  if [[ ${#want[@]} -gt 0 ]] && ! printf '%s\n' "${want[@]}" | grep -qx "$name"; then continue; fi
  CF02_I_WORKER=1 CF02_I_SPEC="$spec" xvfb-run -a -s '-screen 0 1280x720x24' bash "$0" "$out" &
  pids+=("$!")
  # xvfb-run -a picks a free display number: starting two at once can collide.
  sleep 4
  while [[ $(jobs -r | wc -l) -ge "${CF02_I_JOBS:-2}" ]]; do sleep 1; done
done
for pid in "${pids[@]}"; do wait "$pid"; done
