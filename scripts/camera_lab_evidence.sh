#!/usr/bin/env bash
# CAMERA-LAB evidence: real Bevy captures, videos and frame-time probes.
# Run from the repository root after `cargo build` (and `--release` for perf),
# scripts/export_assets.sh and scripts/camera_lab_assets.sh.
#
#   camera_lab_evidence.sh shots  OUT   6 rooms x 4 coverages (+auto) x 3 outputs x solo / 2P separated
#   camera_lab_evidence.sh videos OUT   frame sequences of the headline scenarios (mp4 / gif)
#   camera_lab_evidence.sh perf   OUT   frame time per room x coverage (profile via BIN)
#
# Rendering runs inside xvfb-run like the repository CI scripts. On a machine
# with a GPU the Vulkan adapter is still the GPU; the adapter line is recorded.
set -euo pipefail
mode="${1:?usage: camera_lab_evidence.sh shots|videos|perf OUT}"
out="${2:?output directory}"
bin="${BIN:-./target/debug/camera_lab}"
mkdir -p "$out"

coverages=(1 1.25 1.5 1.75 auto)
outputs=(1920x1080 3840x2160 1280x800)
# Tick of the solo scripted action shot per room (a hit, a spit, a crush, a charge).
solo_tick=(326 186 574 238 316 293)
# Where Donut stands for the 2P separated shot (Carl at his spawn).
sep_x=(805 795 810 820 840 1140)

shot() { # room kind cov output
  local room="$1" kind="$2" cov="$3" res="$4" dir name
  name="R${room}_${kind}_${cov}_${res}"
  dir="$out/$name"
  if [[ "$kind" == solo ]]; then
    args=(--mode solo --script --capture-sequence "${solo_tick[$((room-1))]}:1:1")
  else
    args=(--mode 2p --idle --separate "${sep_x[$((room-1))]}" --capture-sequence "150:1:1")
  fi
  xvfb-run -a -s "-screen 0 ${res}x24" "$bin" --room "$room" --coverage "$cov" --output "$res" \
    "${args[@]}" --capture-dir "$dir" > "$out/$name.log" 2>&1
  grep -q LAB_CAPTURE_SEQUENCE_DONE "$out/$name.log" || { echo "capture failed: $name" >&2; tail -n 5 "$out/$name.log" >&2; return 1; }
  grep -qE 'LAB_LIGHTING_WARNING|LAB_ASSET_FAILURE' "$out/$name.log" && { echo "warning in $name" >&2; return 1; }
  mv "$dir"/frame_*.png "$out/$name.png"
  rmdir "$dir"
  echo "SHOT_OK $name"
}
export -f shot
export out bin
export solo_tick_s="${solo_tick[*]}" sep_x_s="${sep_x[*]}"

case "$mode" in
  shots)
    jobs=()
    for res in "${outputs[@]}"; do
      for room in 1 2 3 4 5 6; do
        for cov in "${coverages[@]}"; do
          for kind in solo sep; do
            jobs+=("$room $kind $cov $res")
          done
        done
      done
    done
    # shellcheck disable=SC2016
    printf '%s\n' "${jobs[@]}" | xargs -P "${JOBS:-6}" -L 1 bash -c '
      solo_tick=($solo_tick_s); sep_x=($sep_x_s)
      shot "$0" "$1" "$2" "$3"'
    ;;
  videos)
    # name room mode flags start step count coverage
    vid() {
      local name="$1" room="$2" cov="$3" start="$4" step="$5" count="$6"; shift 6
      xvfb-run -a -s "-screen 0 1920x1080x24" "$bin" --room "$room" --coverage "$cov" --output 1920x1080 "$@" \
        --capture-sequence "${start}:${step}:${count}" --capture-dir "$out/$name" > "$out/$name.log" 2>&1
      grep -q LAB_CAPTURE_SEQUENCE_DONE "$out/$name.log"
      ffmpeg -y -loglevel error -framerate $((60 / step)) -pattern_type glob -i "$out/$name/frame_*.png" \
        -vf "scale=1280:-2:flags=neighbor" -c:v libx264 -pix_fmt yuv420p -crf 23 "$out/$name.mp4"
      echo "VIDEO_OK $name"
    }
    # Combat hit: the first goblin kill of ROOM-01 (2P script), 1.0x and 1.5x.
    vid combat_hit_room01_1.0 1 1 490 1 100 --mode 2p --script
    vid combat_hit_room01_1.5 1 1.5 490 1 100 --mode 2p --script
    # Destruction cause -> effect: ROOM-03 support, falling weight, crushed goblin.
    vid destruction_room03_1.0 3 1 520 1 120 --mode 2p --script
    vid destruction_room03_auto 3 auto 520 1 120 --mode 2p --script
    # 2P separation and regroup: ROOM-05, automatic camera.
    vid separation_room05_auto 5 auto 420 6 120 --mode 2p --script
    vid separation_room05_1.0 5 1 420 6 120 --mode 2p --script
    # Boss charge into the closed gate: ROOM-06.
    vid boss_charge_room06_auto 6 auto 10 1 150 --mode 2p --script
    vid boss_charge_room06_1.5 6 1.5 10 1 150 --mode 2p --script
    ;;
  perf)
    for room in 1 2 3 4 5 6; do
      for cov in "${coverages[@]}"; do
        for res in ${PERF_OUTPUTS:-1920x1080}; do
          xvfb-run -a -s "-screen 0 ${res}x24" "$bin" --room "$room" --coverage "$cov" --output "$res" \
            --mode 2p --script --perf-frames "${PERF_FRAMES:-240}" > "$out/perf_R${room}_${cov}_${res}.log" 2>&1
          grep -h LAB_PERF "$out/perf_R${room}_${cov}_${res}.log"
        done
      done
    done | tee "$out/perf_summary.txt"
    grep -h -m1 AdapterInfo "$out"/perf_R1_1_*.log > "$out/adapter.txt" || true
    ;;
  *)
    echo "unknown mode: $mode" >&2
    exit 2
    ;;
esac
