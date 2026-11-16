#!/usr/bin/env bash
set -euo pipefail
mkdir -p artifacts
./target/debug/dungeon-floorbreak > artifacts/game.log 2>&1 &
game_pid=$!
cleanup() {
  kill "$game_pid" 2>/dev/null || true
  wait "$game_pid" 2>/dev/null || true
}
trap cleanup EXIT

window_seen=0
for _ in $(seq 1 30); do
  if ! kill -0 "$game_pid" 2>/dev/null; then
    cat artifacts/game.log
    echo "ERROR: game exited before producing a window" >&2
    exit 1
  fi
  if xwininfo -root -tree | grep -Fq 'The Crawl |'; then
    window_seen=1
    break
  fi
  sleep 1
done

if [[ "$window_seen" -ne 1 ]]; then
  cat artifacts/game.log
  echo "ERROR: no Bevy game window found" >&2
  exit 1
fi
sleep 5
if ! kill -0 "$game_pid" 2>/dev/null; then
  cat artifacts/game.log
  echo "ERROR: game died after opening its window" >&2
  exit 1
fi
import -window root artifacts/render-smoke.png
image_size=$(identify -format '%wx%h' artifacts/render-smoke.png)
unique_colors=$(identify -format '%k' artifacts/render-smoke.png)
if [[ "$image_size" != "1280x720" || "$unique_colors" -lt 3 ]]; then
  cat artifacts/game.log
  echo "ERROR: unexpected screenshot size or effectively blank frame: $image_size, $unique_colors colors" >&2
  exit 1
fi
printf 'RENDER_SMOKE_OK: Linux window rendered %s (%s distinct colors)\n' "$image_size" "$unique_colors"
