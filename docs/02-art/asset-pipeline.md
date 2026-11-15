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

## WORK-008 implementation spike — original editable fixtures

**Status:** implementation PR in progress; no human pixel-art quality approval.

Editable Aseprite-native `.ase` sources are committed and compatible with
Aseprite's documented binary format. For CI we use the **open-source LibreSprite
1.3** Aseprite-compatible batch exporter (the official GitHub release is pinned
to SHA-256 `fd90bddda8135fe00f4a9a9c8c9ebd48f9b07e8782ab269be5c9efb1a851af30`).
This avoids shipping, redistributing or requiring a proprietary Aseprite binary.
Artists may still open/edit the native .ase files in Aseprite.

The initial deliberately small original-IP-neutral files are:

- `assets/source/terrain/dungeon/materials.ase`: one editable
  64×16 source containing **four 16×16 material cells** (stone, breakable
  wood, reinforced metal and glass).
- `assets/source/props/fuse_barrel.ase`: four tagged original
  16×24 prop frames to verify animation export and alpha.
- `assets/source/characters/placeholder/worker.ase`: four tagged
  32×52 frames of a **generic unnamed neutral actor**, explicitly **not Carl**.
  No shotgun or character likeness is intended.

The exported PNGs are intentionally **not committed**; they are produced from
editable .ase masters by a pinned CLI and checked in CI. Each fixture has a
provenance/pivot/size/tag contract in `assets/manifests/*.json`.

### Local reproduction (Linux)

```sh
python3 -m pip install Pillow==13.0.0
# Install a lawful Aseprite or LibreSprite binary first. With LibreSprite 1.3:
export ART_CLI=/path/to/libresprite
bash scripts/export_assets.sh
python3 -m unittest discover -s tests -p 'test_art_assets.py' -v
```

GitHub Actions `.github/workflows/art.yml` fetches **LibreSprite v1.3**,
verifies its official upstream SHA-256, extracts the AppImage without FUSE,
performs exports, runs structural validators and negative tests, verifies
**identical PNG/JSON checksums after a second export**, and uploads the result
as the `art-work008-review` workflow artifact:

- `generated/art/dungeon_materials.{png,json}`
- `generated/art/fuse_barrel.{png,json}`
- `generated/art/neutral_worker.{png,json}`
- `generated/art/review_640x360.png` (native resolution)
- `generated/art/review_2x.png` (integer scaling)

**Scope boundary:** This establishes the pipeline and three tiny editable
fixtures, **not** polished game art. The contact sheet is a composition study,
not a Bevy screenshot, actual platform collision or gameplay proof. Work
cannot pass the WORK-007 human-art gate until a native-resolution scene
has been reviewed separately. No recognizable fan character sources or
concept-image crops are emitted here.
