#!/usr/bin/env bash
# PLAYTRACE-RENDER-01 sim-pinned replay build (design section 3-C, PTR-Q16).
#
# Usage: scripts/replay_session_pinned.sh <commit> <session-id> [floor01_scene args...]
#
# Builds the replay viewer at <commit> in a temporary detached git worktree and
# replays <session-id> from the default session store in checked mode. <commit>
# must contain the viewer and have the same simulation sources as the build that
# recorded the session (for sessions recorded at 0bffd264: 12e43398).
# The worktree is local, never pushed, and removed on exit. Run it from the
# checkout that holds the exported art (generated/); run scripts/export_assets.sh
# there first if the directory is missing. Set CARGO_TARGET_DIR to reuse a build
# cache across runs. No process is killed by this script.
set -euo pipefail

if [ "$#" -lt 2 ]; then
    echo "usage: $0 <commit> <session-id> [floor01_scene args...]" >&2
    exit 2
fi
commit=$1
session=$2
shift 2

root=$(git rev-parse --show-toplevel)
if [ ! -d "$root/generated/art" ]; then
    echo "missing $root/generated/art: run scripts/export_assets.sh first" >&2
    exit 2
fi
if ! git -C "$root" rev-parse --verify --quiet "$commit^{commit}" >/dev/null; then
    echo "unknown commit: $commit (try git fetch origin)" >&2
    exit 2
fi

dir=$(mktemp -d "${TMPDIR:-/tmp}/floorbreak-pinned.XXXXXX")
cleanup() {
    git -C "$root" worktree remove --force "$dir" >/dev/null 2>&1 || rm -rf "$dir"
}
trap cleanup EXIT
git -C "$root" worktree add --detach "$dir" "$commit" >/dev/null
cp -r "$root/generated" "$dir/generated"

cd "$dir"
cargo run --locked --release --bin floor01_scene -- \
    --replay-session "$session" --replay-compatibility checked --scale 3 "$@"
