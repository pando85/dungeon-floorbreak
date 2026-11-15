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
- No third-party character likenesses, protected dialogue, trademarks used decoratively or copyrighted game assets enter this public repo without permission. Fan-concept character art (Carl, Donut, Mongo) in concept and production form is covered by the owner-reported `ART-PUBLICATION-ALL-V1` record in the [rights register](../04-production/rights-and-release.md); it still needs manifest/provenance/checksum entries and grants no reuse licence or game/asset-pack distribution right.

## Source ownership

Lock canonical characters and shared palette through explicit approval. Agents may branch experimental variations but should not silently overwrite masters. Prefer one agent responsible for integrating a sprite family to avoid cross-agent style drift.

## Optional Mewgenics-inspired workflow

Create face/tail/accessory modules that can be exchanged at runtime or layered into exported frames. The inspiration is **modularity**, not cloning any proprietary rigs or visuals. For every modular animation compare pixel-shimmer against a hand-pixeled baseline; select case by case.

## WORK-008 implementation spike — original editable fixtures

**Status:** functional export pipeline verified in PR #13 CI (all automated jobs green); human pixel-art quality approval remains separate.

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

### Tagged animation previews (follow-up)

The pipeline additionally generates a small **looping GIF per animation tag**
using the exported source frame durations. Previews are intentionally generated
from native editable .ase sources, not from concept-art crops or interpolated
AI-generated frames. They are displayed at nearest-neighbor 6× scale against a
neutral dark background to expose pixel jitter and drifting silhouettes.

- Script: `scripts/make_animation_previews.py`
- Output: `generated/art/previews/<asset-id>-<tag>.gif` (automatically uploaded
  with the `art-work008-review` GitHub Actions artifact)
- Tests: `tests/test_art_animation_previews.py` cover durations, two-frame loops,
  illegal paths, tag bounds and atlas bounds.
- Restrictions: not a full animation-quality approval; inspect actual source at
  gameplay scale alongside the native 640×360 contact sheet.

### Text pixel-grid sources (`pixel-grid-v1`) and gameplay-scale comparison

Pixel sprites can be authored as diff-friendly text and converted to the native
`.ase` master by `scripts/pixel_grid_to_ase.py` (deterministic, no timestamps;
written from the official Aseprite file specification, RGBA, named layers, frame
durations, tags chunk, zlib cels and the **named palette** in a palette chunk).

```json
{"format": "pixel-grid-v1", "canvas": [W, H], "pivot": [x, y],
 "palette": [{"key": "K", "name": "outline", "rgba": "#1b1222ff"}],
 "layers": ["body", "head"],
 "frames": [{"duration_ms": 180, "cels": {"body": ["....", "..K."]}}],
 "tags": [{"name": "idle", "from": 0, "to": 3, "direction": "forward"}]}
```

- `layers` run bottom to top; each cel is exactly `H` strings of `W` characters; a
  layer missing from a frame is an empty cel. `.` is transparent, every other
  character must be a single-character palette key. Palette alpha must be `ff`
  (no antialiasing or partial alpha). Bad row lengths, unknown keys, duplicate
  keys or layers, a pivot outside the canvas and tag ranges are rejected.
- The pivot is not stored in a way LibreSprite reads (its 1.3 loader rejects the
  slice chunk), so the manifest `pivot` is authoritative; the `.ase` carries it as
  layer user-data text for traceability. Pivot y is the bottom opaque row.
- `python3 scripts/pixel_grid_to_ase.py lint GRID.json [--strict]
  [--expect-height N]` reports frame-0 silhouette height and bbox, contact row
  versus pivot, colours used, orphan single pixels and bottom-row drift across
  frames. `render GRID.json --out DIR [--scale N] [--bg #rrggbb]` writes
  nearest-neighbour PNGs per frame for quick review. `build GRID.json OUT.ase`
  converts.
- **Manifest flow:** `scripts/export_assets.sh` iterates
  `assets/manifests/*.json`. A manifest with `grid_source` has its `source_path`
  `.ase` rebuilt from that grid first, then the same LibreSprite export writes
  `output_atlas`/`output_metadata` under `generated/art/`. Validation, contact
  sheet and GIF previews then run for every manifest.
- **CI rule:** the generated `.ase` is committed next to its grid. After export,
  CI fails if `git status --porcelain -- assets/source` is not empty, so after
  editing a grid run `bash scripts/export_assets.sh` and commit the regenerated
  `.ase`. `tests/test_art_pixel_grid.py` also checks committed `.ase` equals a
  fresh build.
- Public IP-neutral fixtures: `neutral_blob` (pipeline check) and
  `height_mannequin` (plain 52 px human silhouette, no face, costume or weapon,
  **not Carl**; the middle of Carl's provisional 48 to 56 px range).
  `neutral_worker` has a 32×52 frame but only **47 px** of opaque height, so it is
  not a 52 px height reference.

**Scale comparison.** `scripts/make_scale_comparison.py --sprite
LABEL=ATLAS.png:META.json[:FRAME]` (1 to 4 times, optional `--reference`, default
the 52 px mannequin labelled "Carl-height proxy (neutral 52 px mannequin, NOT
Carl)") puts all sprites on one floor line, camera and lighting in the existing
gothic/violet scene and writes into `--out`: `scene_640x360.png` (1x, clean),
`scene_640x360_annotated.png` (dotted guides at 48 px cyan and 56 px yellow),
`scene_1280x720_2x.png`, Steam Deck framing `scene_640x400.png` and
`scene_1280x800_2x.png` (same floor, 40 extra rows above), 6x nearest close-ups
`closeup_<label>_6x.png` and `_6x_grid.png`, `closeups_side_by_side_6x.png`,
`comparison_sheet.png` (labels only in bands outside the scene) and
`comparison.json` (measured opaque height, width, pivot and colours). The
export script runs it into `generated/art/scale_study/` when
`assets/manifests/donut_scale_h*.json` exist. It measures sprites; the artistic
decision stays with human review.

**Local ART_CLI setup (pinned, no sudo).**

```sh
mkdir -p ~/.cache/art-cli && cd ~/.cache/art-cli
curl -fLo libresprite.AppImage https://github.com/LibreSprite/LibreSprite/releases/download/v1.3/libresprite-v1.3-linux-x86_64.AppImage
echo 'fd90bddda8135fe00f4a9a9c8c9ebd48f9b07e8782ab269be5c9efb1a851af30  libresprite.AppImage' | sha256sum -c -
chmod +x libresprite.AppImage && ./libresprite.AppImage --appimage-extract >/dev/null
export ART_CLI="$PWD/squashfs-root/AppRun"   # then: bash scripts/export_assets.sh
```

A distribution LibreSprite (for example a locally built "1.2-dev") produced
byte-identical PNGs; its JSON differs only in `meta.version`. CI uses the pinned
AppImage, so keep that as the reference.

**Scope boundary:** This establishes the pipeline and three tiny editable
fixtures, **not** polished game art. The contact sheet is a composition study,
not a Bevy screenshot, actual platform collision or gameplay proof. Work
cannot pass the WORK-007 human-art gate until a native-resolution scene
has been reviewed separately. No recognizable fan character sources or
concept-image crops are emitted by this pipeline stage; character production art
is published separately under the `ART-PUBLICATION-ALL-V1` scope in the
[rights register](../04-production/rights-and-release.md).
