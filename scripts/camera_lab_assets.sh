#!/usr/bin/env bash
# CAMERA-LAB: export the Ball of Swine boss atlas (SWINE-01) that has no entry in
# assets/manifests yet, so the lab can draw the approved art. Run after
# scripts/export_assets.sh. Writes generated/art/swine_01.{png,json}; nothing
# under assets/ is changed. The pivot and frame size come from the committed
# pixel-art asset file assets/source/characters/swine/swine-01.pixelart.json.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ART_CLI="${ART_CLI:-}"
if [[ -z "$ART_CLI" ]]; then
  for program in aseprite libresprite; do
    command -v "$program" >/dev/null 2>&1 && { ART_CLI="$program"; break; }
  done
fi
[[ -n "$ART_CLI" ]] || { echo 'Missing Aseprite/LibreSprite CLI (set ART_CLI).' >&2; exit 2; }
src=assets/source/characters/swine/swine-01.ase
[[ -f "$src" ]] || { echo "Missing $src" >&2; exit 2; }
mkdir -p generated/art
"$ART_CLI" -b --list-tags "$src" \
  --sheet generated/art/swine_01.png \
  --data generated/art/swine_01.json \
  --format json-array --sheet-type rows
[[ -s generated/art/swine_01.png && -s generated/art/swine_01.json ]]
echo "CAMERA_LAB_ASSETS_OK swine_01"
