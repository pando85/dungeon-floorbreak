#!/usr/bin/env bash
# Source this file: it defines `prepare_captures_dir DIR`, the guard in front of the one destructive step of
# an evidence script (the removal of its scratch capture directory). Run from the repository root.
#
# DIR is (re)created empty, with a marker file, only when ALL of these hold; otherwise nothing is removed and
# the function returns 2:
#   * it is not empty and not a symlink;
#   * it resolves to a path strictly below ./artifacts inside the repository (so it is never the repository
#     root, an ancestor of it, ./artifacts itself, "." or "/", nor a path outside the repository);
#   * if it already exists, it is a directory that holds the marker the guard writes itself (a dedicated
#     capture directory this script made before), so a directory somebody else filled is never deleted.
prepare_captures_dir() {
  local dir="${1-}" marker=".capture-dir" root abs
  if [[ -z "$dir" ]]; then
    echo "refusing an empty captures directory" >&2
    return 2
  fi
  if [[ -L "$dir" ]]; then
    echo "refusing captures directory '$dir': it is a symlink" >&2
    return 2
  fi
  root="$(pwd -P)"
  abs="$(realpath -m -- "$dir")"
  case "$abs" in
    "$root/artifacts/"?*) ;;
    *)
      echo "refusing captures directory '$dir': it must be a directory below ./artifacts of this repository" >&2
      return 2
      ;;
  esac
  if [[ -e "$abs" ]]; then
    if [[ ! -d "$abs" || ! -f "$abs/$marker" ]]; then
      echo "refusing captures directory '$dir': it exists and is not a dedicated capture directory (no $marker marker)" >&2
      return 2
    fi
    rm -rf -- "$abs"
  fi
  mkdir -p -- "$abs"
  : > "$abs/$marker"
}
