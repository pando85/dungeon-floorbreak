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
# Manifest-driven: every assets/manifests/*.json names its editable .ase source
# and export targets. A manifest with "grid_source" (pixel-grid-v1 JSON) first
# has its .ase rebuilt deterministically from that text grid.
for manifest in assets/manifests/*.json; do
  mapfile -t fields < <(python3 -I - "$manifest" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
for key in ("source_path", "output_atlas", "output_metadata"):
    if not isinstance(data.get(key), str) or not data[key]:
        sys.exit(f"{sys.argv[1]}: missing {key}")
print("\n".join([data["source_path"], data["output_atlas"], data["output_metadata"],
                  data.get("grid_source") or ""]))
PY
  )
  source_file="${fields[0]}"
  output_atlas="${fields[1]}"
  output_metadata="${fields[2]}"
  grid_source="${fields[3]}"
  if [[ -n "$grid_source" ]]; then
    python3 scripts/pixel_grid_to_ase.py build "$grid_source" "$source_file"
  fi
  "$ART_CLI" -b --list-tags "$source_file" \
    --sheet "$output_atlas" \
    --data "$output_metadata" \
    --format json-array --sheet-type rows
  [[ -s "$output_atlas" && -s "$output_metadata" ]] || {
    echo "Missing export files for $manifest" >&2
    exit 1
  }
done
python3 scripts/validate_assets.py
python3 scripts/make_contact_sheet.py
python3 scripts/make_animation_previews.py
# Gameplay-scale sprite comparison, only when scale-study manifests exist.
scale_args=()
for manifest in assets/manifests/donut_scale_h*.json; do
  [[ -e "$manifest" ]] || continue
  mapfile -t fields < <(python3 -I - "$manifest" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
print("\n".join([data["id"], data["output_atlas"], data["output_metadata"]]))
PY
  )
  scale_args+=(--sprite "${fields[0]}=${fields[1]}:${fields[2]}")
done
if [[ ${#scale_args[@]} -gt 0 ]]; then
  python3 scripts/make_scale_comparison.py "${scale_args[@]}" --out generated/art/scale_study
fi
