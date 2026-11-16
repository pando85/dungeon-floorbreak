#!/usr/bin/env bash
# Run inside xvfb-run with the target display resolution.
set -euo pipefail
mode="${1:-desktop}"
mkdir -p artifacts
case "$mode" in
  desktop) suffix="640x360"; expected="1280x720"; args=() ;;
  deck) suffix="640x400"; expected="1280x800"; args=(--deck) ;;
  *) echo "Unknown capture mode: $mode" >&2; exit 2 ;;
esac
./target/debug/character_playground "${args[@]}" > "artifacts/character-playground-${suffix}.log" 2>&1 &
pid=$!
cleanup() {
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
}
trap cleanup EXIT
for _ in $(seq 1 40); do
  if ! kill -0 "$pid" 2>/dev/null; then
    cat "artifacts/character-playground-${suffix}.log"
    echo "Playground exited unexpectedly" >&2
    exit 1
  fi
  if grep -q PLAYGROUND_TEXTURES_READY "artifacts/character-playground-${suffix}.log"; then
    break
  fi
  sleep 1
done
if ! grep -q PLAYGROUND_TEXTURES_READY "artifacts/character-playground-${suffix}.log"; then
  cat "artifacts/character-playground-${suffix}.log"
  echo "Character textures did not load" >&2
  exit 1
fi
if ! xwininfo -root -tree | grep -Fq 'THE CRAWL - Character Playground'; then
  echo "Character Playground window missing" >&2
  exit 1
fi
sleep 2
import -window root "artifacts/character-playground-${suffix}.png"
actual="$(identify -format '%wx%h' "artifacts/character-playground-${suffix}.png")"
colors="$(identify -format '%k' "artifacts/character-playground-${suffix}.png")"
if [[ "$actual" != "$expected" || "$colors" -lt 16 ]]; then
  echo "Bad screenshot: $actual, $colors colors" >&2
  exit 1
fi
echo "PLAYGROUND_RENDER_OK: $suffix at $actual ($colors colors)"
