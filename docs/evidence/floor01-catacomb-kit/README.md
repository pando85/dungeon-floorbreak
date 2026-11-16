# Floor 01 catacomb environment kit (ENV-01): evidence images

These are visual review aids for the ENV-01 environment kit proposal described in [`docs/02-art/floor-01-catacomb-kit.md`](../../02-art/floor-01-catacomb-kit.md). They are **art previews composited by a script from the exported atlases**. They are **not Bevy screenshots**, not collision or gameplay proof, and not approvals. A human art review is pending. Real in-engine evidence is the job of COMP-01.

- **Source:**
  - branch `art/env-01-floor01-catacomb-kit`;
  - kit contract `assets/source/terrain/floor01/floor01_catacomb_kit.json`;
  - sample layout `assets/levels/floor01/corridor_abc_sample.json`.
- **Renderer:** `scripts/env_kit.py all`, run by `scripts/export_assets.sh`. The renderer only pastes rects from `generated/art/floor01_*.png` in the contract's draw order. It adds no lighting, filtering or hand painting.
- **Integrity:** `SHA256SUMS` in this directory lists the SHA-256 of every image, and `python3 scripts/check_docs.py` verifies it. The images were rendered locally with Pillow 12.3 and a distribution LibreSprite build. The [pipeline notes](../../02-art/asset-pipeline.md) report byte-identical atlas PNGs between a distribution build and the pinned LibreSprite 1.3, and CI re-exports everything from the sources. PNG bytes may still differ across Pillow versions even when the pixels are identical.
- **Rights:**
  - The environment art is original agent-authored pixel art (Claude Opus).
  - The concept panel in the side-by-side is the committed mood reference [`01-catacomb-environment-clean.png`](../../../assets/concept/floor-01/opening/01-catacomb-environment-clean.png). It is only display-scaled and was never used as a pixel source.
  - The `actor_check_*` images composite the approved Carl 52 px and Donut 40 px atlases under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence, and grants no game or asset-pack distribution right.

## Regenerating

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
# previews land in generated/art/env_kit/floor01_corridor_abc_sample/
```

## Files

| File | Content | Regenerate from |
| --- | --- | --- |
| `corridor_abc_640x360.png` | Clean A → B → C corridor composition, native 640×360 (view `abc_640x360` of the sample layout) | `generated/art/env_kit/floor01_corridor_abc_sample/corridor_abc_640x360.png` |
| `corridor_abc_1280x720_2x.png` | The same view, integer nearest-neighbour 2× | `…/corridor_abc_1280x720_2x.png` |
| `corridor_abc_640x400.png` | 16:10 framing of the same columns (view `abc_640x400`) | `…/corridor_abc_640x400.png` |
| `corridor_abc_1280x800_2x.png` | 16:10 framing, nearest 2× | `…/corridor_abc_1280x800_2x.png` |
| `corridor_overview_2048x400.png` | The whole 128×25-tile sample layout at 1×: segments A1, B1, C1, A2, B2 and the raised block | `…/corridor_overview_2048x400.png` |
| `kit_sheet_1x.png` | Exploded contact sheet of every tile, piece and torch frame, native 1× | `…/kit_sheet_1x.png` |
| `kit_sheet_2x.png` | The same sheet, nearest 2× | `…/kit_sheet_2x.png` |
| `corridor_vs_concept_side_by_side.png` | Left: composition at 2×. Right: concept 01, display-scaled to the same size. Labelled; mood comparison only | `…/corridor_vs_concept_side_by_side.png` |
| `actor_check_640x360.png` | Contrast check: approved Carl 52 px and Donut 40 px idle frame 0 placed on the floor in A, B (in front of the recess and far arcade) and C, with the owner-chosen 0 px foot sink (pivot row on the collider top). Not Bevy | `…/actor_check_640x360.png` |
| `actor_check_1280x720_2x.png` | The contrast check, nearest 2× | `…/actor_check_1280x720_2x.png` |
| `floor01_torch_v1-burn.gif` | The 4-frame torch loop (120/140/100/130 ms) at 6×, written by `scripts/make_animation_previews.py` | `generated/art/previews/floor01_torch_v1-burn.gif` |
