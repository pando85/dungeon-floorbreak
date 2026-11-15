# Reproducible art pipeline (AI-first)

## Goals

All visual assets have editable source, generated build output, metadata, provenance, deterministic regeneration where possible and a quick way for agents to inspect before merge. The pipeline must work on Linux in CI without a GUI for normal export tasks.

## WORK-008 implementation — headless CLI and CI

**Implementation status:** pipeline tooling exists in `scripts/art_pipeline.py`; a real
editor export is executed on each PR by `.github/workflows/art.yml`. A tiny,
original, neutral two-frame `.aseprite` file is included exclusively for
tests. **No Donut or Carl production sprites exist yet.** The actual
character source/art and visual acceptance are separate future work.

The CLI accepts Aseprite or compatible LibreSprite. CI uses a pinned LibreSprite
1.3 AppImage with a recorded upstream SHA-256 (no proprietary Aseprite license
required on GitHub runners). The CLI is the export automation; custom
Aseprite Lua generation is intentionally deferred until authoring operations
require scripts, rather than shipping untested Lua.

Install Python 3.12 and Pillow 12.2.0 (CI pins the version). With an editor
available locally (set `ASEPRITE_BIN` if not named `aseprite`):

```sh
python3 -m pip install 'Pillow==12.2.0'
python3 -m unittest discover -s tests -p 'test_art_pipeline.py' -v
python3 scripts/art_pipeline.py export \
  --manifest tests/fixtures/neutral_smoke.manifest.json \
  --editor "$ASEPRITE_BIN"
python3 scripts/art_pipeline.py validate \
  --manifest tests/fixtures/neutral_smoke.manifest.json
```

Set `ASEPRITE_BIN=/path/to/aseprite` or supply `--editor /path/to/libresprite`
for a local export. CI calls the same exporter with LibreSprite's extracted
`AppRun`. Outputs are never committed; they reside under `generated/`
and evidence under `artifacts/art/`. The workflow uploads the PNG atlas,
Aseprite JSON array, a 6× nearest-neighbor contact sheet, looping GIF,
validation report and repeat-export hash evidence.

### Contract enforced by the exporter

- Input is an **editable** `.aseprite` or `.ase` source with named timeline tags,
  accompanied by a JSON v1 manifest; no generated concept sheet is accepted
  as sprite source.
- Export is untrimmed and unrotated, with 2 px between sheet cells to avoid
  atlas bleeding; every frame has a valid positive duration.
- Every frame must be covered by exactly one tag, fit inside the atlas, have
  the requested dimensions and contain visible pixels. Transparent borders
  are required when `required_alpha` is true.
- Palette color budget is optional and explicit; source and atlas paths are
  restricted to repository-safe locations.
- Frame-local pivots are documented as **one fixed pixel pivot per source
  frame**; character movement/hitbox reconciliation remains a separate
  runtime step (WORK-012/animation QA). No false claim of per-frame
  alignment verification is made.
- A second export at the same path must produce **identical PNG and JSON
  checksums**. A non-repeatable export fails CI.

**Next agent deliverable:** replace the neutral fixture with a real Aseprite
Donut `caster` prototype only after art approval, native-scale readability
tests and applicable rights review; keep the original approved illustrated
concept in its reference folder.

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
