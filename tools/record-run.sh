#!/usr/bin/env bash
# record-run.sh — Run the game via cargo run and record its window as a video.
#
# Usage:
#   ./tools/record-run.sh <binary> [game args...]
#
# Examples:
#   ./tools/record-run.sh floor01_scene --demo --scale 3
#   ./tools/record-run.sh floor01_scene --fixture cp01_single_goblin
#   ./tools/record-run.sh character_playground
#   ./tools/record-run.sh camera_lab
#
# Recordings are saved to: recordings/session_YYYYMMDD_HHMMSS.mp4
set -euo pipefail

# --- Config ---
WINDOW_TITLE="The Crawl"  # partial match via xdotool
OUTDIR="recordings"
mkdir -p "$OUTDIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTFILE="$OUTDIR/session_${TIMESTAMP}.mp4"
STARTUP_DELAY=5  # seconds to wait for game window to appear

# --- Check args ---
if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <binary> [game args...]" >&2
  echo "Example: $0 floor01_scene --demo --scale 3" >&2
  exit 1
fi

BIN_NAME="$1"
shift

# --- Check dependencies ---
for cmd in ffmpeg xdotool; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "Error: '$cmd' is required but not installed." >&2
    exit 1
  fi
done

# --- Start the game ---
echo "Starting: cargo run --release --locked --bin $BIN_NAME -- $*"
cargo run --release --locked --bin "$BIN_NAME" -- "$@" &
GAME_PID=$!

# --- Wait for window to appear ---
echo "Waiting ${STARTUP_DELAY}s for game to start..."
sleep "$STARTUP_DELAY"

# --- Find the game window (partial title match) ---
WINDOW_ID=$(xdotool search --name "$WINDOW_TITLE" 2>/dev/null | head -1 || true)
if [[ -z "$WINDOW_ID" ]]; then
  echo "Error: Window matching '$WINDOW_TITLE' not found." >&2
  kill "$GAME_PID" 2>/dev/null || true
  exit 1
fi

echo "Window found (ID: $WINDOW_ID). Starting recording..."

# --- Get window geometry for region capture (more reliable than -window_id) ---
WIN_GEOM=$(xdotool getwindowgeometry --shell "$WINDOW_ID" 2>/dev/null || true)
if [[ -n "$WIN_GEOM" ]]; then
  eval "$WIN_GEOM"
  WIN_X=$X
  WIN_Y=$Y
  WIN_W=$WIDTH
  WIN_H=$HEIGHT
  echo "Window geometry: ${WIN_W}x${WIN_H}+${WIN_X}+${WIN_Y}"
else
  echo "Warning: Could not get window geometry, falling back to -window_id"
  WIN_X=""
fi

# --- Start FFmpeg recording ---
FFMPEG_LOG="$OUTDIR/ffmpeg_${TIMESTAMP}.log"
echo "Starting ffmpeg recording (log: $FFMPEG_LOG)..."

# Get the default sink's monitor source for audio capture
AUDIO_SOURCE=$(pactl info 2>/dev/null | grep "Default Sink" | awk '{print $3}').monitor
echo "Using audio source: $AUDIO_SOURCE"

if [[ -n "$WIN_X" ]]; then
  # Region capture: more reliable than -window_id
  ffmpeg -y \
    -f x11grab \
    -framerate 60 \
    -video_size "${WIN_W}x${WIN_H}" \
    -i "$DISPLAY+$WIN_X,$WIN_Y" \
    -f pulse \
    -i "$AUDIO_SOURCE" \
    -c:v libx264 -preset ultrafast -crf 18 \
    -c:a aac -b:a 192k \
    -pix_fmt yuv420p \
    "$OUTFILE" 2>"$FFMPEG_LOG" &
else
  # Fallback to window_id
  ffmpeg -y \
    -f x11grab \
    -window_id "$WINDOW_ID" \
    -framerate 60 \
    -i "$DISPLAY" \
    -f pulse \
    -i "$AUDIO_SOURCE" \
    -c:v libx264 -preset ultrafast -crf 18 \
    -c:a aac -b:a 192k \
    -pix_fmt yuv420p \
    "$OUTFILE" 2>"$FFMPEG_LOG" &
fi
FFMPEG_PID=$!

# --- Wait for the game to exit ---
echo "Recording... (game PID: $GAME_PID)"
wait "$GAME_PID" || true
EXIT_CODE=$?

# --- Stop recording ---
echo "Game exited. Stopping recording..."
kill -INT "$FFMPEG_PID" 2>/dev/null || true
wait "$FFMPEG_PID" 2>/dev/null || true

# --- Verify recording ---
sleep 1  # Give ffmpeg time to finalize the file
if [[ -f "$OUTFILE" ]]; then
  FILE_SIZE=$(stat -c%s "$OUTFILE" 2>/dev/null || echo 0)
  if [[ "$FILE_SIZE" -gt 1000 ]]; then
    echo "✓ Recording saved to: $OUTFILE ($FILE_SIZE bytes)"
  else
    echo "⚠ Recording file is empty or too small ($FILE_SIZE bytes)"
    echo "FFmpeg log:"
    cat "$FFMPEG_LOG"
  fi
else
  echo "✗ Recording file was not created"
  echo "FFmpeg log:"
  cat "$FFMPEG_LOG"
fi

exit $EXIT_CODE
