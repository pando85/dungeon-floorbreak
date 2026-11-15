# Reproducible art pipeline (AI-first)

## Goals

All visual assets have editable source, generated build output, metadata, provenance, deterministic regeneration where possible and a quick way for agents to inspect before merge. The pipeline must work on Linux in CI without a GUI for normal export tasks.

## Proposed tools

- **Aseprite** with Lua scripting and CLI export (`.aseprite` → sprite sheets + JSON).
- **LDtk** for authored level layouts; JSON validated and normalized for runtime.
- **Python/Pillow or a small Rust CLI** for contact sheets, palette/alpha checks and atlas processing (not a permanent dependency mandate).
- **Bevy** for sprite atlas animation and effects rendering.
- **Optional SVG or Blender** for concept geometry/prototypes; rasterize before final pixel-art editing. Do not rely on a final automatic downscale to make polished pixel art.

## Suggested layout (only create when implementation begins)

```text
assets/
  source/characters/demolitionist/*.aseprite
  source/characters/caster/*.aseprite
  source/characters/charger/*.aseprite
  source/terrain/dungeon/*.aseprite
  source/effects/*.aseprite
  source/levels/*.ldtk
  manifests/*.json
  provenance/*.json
  generated/      # reproducible output, normally ignored
scripts/
  export_assets.sh
  validate_assets.py
  make_contact_sheet.py
```

## Export command example

```sh
aseprite -b assets/source/characters/demolitionist/base.aseprite \
  --sheet generated/demolitionist.png \
  --data generated/demolitionist.json \
  --format json-array
```

The example is **illustrative**; it will not work until the named source file exists. Verify Aseprite version and CLI options in CI before implementation.

## Asset contract (minimum)

Every manifest records: `id`, `type`, `source_path`, `output_atlas`, `palette_id`, `frame_size`, `pivot`, `animation_tags`, `required_alpha`, `owner`, `tool_version`, `provenance`, `license`, `review_status`. Optional `rights_clearance_ref` for third-party-inspired content. See `examples/asset-manifest.example.json`.

## Agent acceptance checks

- Sprite sheet loads and tags map to finite frame ranges.
- No out-of-bounds frame rectangles, negative dimensions, unexpected opaque background, missing required tags.
- Consistent foot/pivot anchor between frames; atlas has 1 px or more extruded padding where filtering requires it.
- Palette and color count within selected character/art constraints (thresholds chosen empirically, not yet defined).
- Every asset has a valid license/provenance declaration; **AI-generated** does not mean unrestricted rights.
- Artifact preview exists: contact sheet + short animation capture; compare at gameplay scale.
- No franchise character likenesses, protected dialogue, trademarks used decoratively or copyrighted game assets enter this public repo without permission.

## Source ownership

Lock canonical characters and shared palette through explicit approval. Agents may branch experimental variations but should not silently overwrite masters. Prefer one agent responsible for integrating a sprite family to avoid cross-agent style drift.

## Optional Mewgenics-inspired workflow

Create face/tail/accessory modules that can be exchanged at runtime or layered into exported frames. The inspiration is **modularity**, not cloning any proprietary rigs or visuals. For every modular animation compare pixel-shimmer against a hand-pixeled baseline; select case by case.
