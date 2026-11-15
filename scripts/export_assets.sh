#!/usr/bin/env bash
# Render editable Aseprite/LibreSprite .ase sources with a real pixel-art CLI.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ART_CLI="${ART_CLI:-}"
if [[ -z "$ART_CLI" ]]; then
  for program in aseprite libresprite; do
    if command -v "$program" >/dev/null 2>&1; then
      ART_CLI="$program"
      break
    fi
  done
fi
if [[ -z "$ART_CLI" ]]; then
  echo 'Missing Aseprite/LibreSprite CLI. Set ART_CLI to the executable path.' >&2
  exit 2
fi
mkdir -p generated/art
for entry in \
  'assets/source/terrain/dungeon/materials.ase:dungeon_materials' \
  'assets/source/props/fuse_barrel.ase:fuse_barrel' \
  'assets/source/characters/placeholder/worker.ase:neutral_worker'; do
  source_file="${entry%%:*}"
  name="${entry#*:}"
  "$ART_CLI" -b --list-tags "$source_file" \
    --sheet "generated/art/${name}.png" \
    --data "generated/art/${name}.json" \
    --format json-array --sheet-type rows
  [[ -s "generated/art/${name}.png" && -s "generated/art/${name}.json" ]] || {
    echo "Missing export files for $name" >&2
    exit 1
  }
done
python3 scripts/validate_assets.py
python3 scripts/make_contact_sheet.py
