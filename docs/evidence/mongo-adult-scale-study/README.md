# Mongo adult scale study: evidence images

Visual evidence for the 64/76/88 px Mongo (adult / mount) native pixel scale study and the 76 px idle prototype. These are **review aids for a human artistic decision**, not approvals. The evaluation and the proposed (not locked) scale are in [`docs/02-art/mongo-scale-study.md`](../../02-art/mongo-scale-study.md).

- **Source commits:** the Mongo grids and manifests are those of `ea1de02` (64/76/88 px masters) plus `c0bae86` (76 px four-frame idle and 13 enclosed pinholes filled), branch `art/mongo-adult-sprite-scale`, stacked on `art/carl-book1-sprite-scale` (PR #19), which is stacked on `art/donut-floor1-scale-study` (PR #18). All pipeline renders below were produced at commit `c0bae865fc267f567861210f2f2ca2bbeb762c28`. The width-aware spacing fallback that lays out these wide actors is commit `e00fc4b`. The companions in the scene are Donut 32 px (Donut study, `027b36a` / `4918b73`) and Carl 52 px (Carl study, `5d00770` / `957270e`).
- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; it grants no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` in this directory lists the SHA-256 of every image. `python3 scripts/check_docs.py` verifies it.
- **Not Carl:** the grey 52 px figure in the scene is a neutral height mannequin proxy for Carl's provisional height, not Carl. Left to right in the scene: mannequin, Donut 32 px, Carl 52 px, Mongo 64, 76, 88 px. Donut and Carl are the real study masters; the mannequin is the only proxy.

## Regenerating the pipeline renders

Run from the repository root. `ART_CLI` is any Aseprite or LibreSprite command-line executable. The first command rebuilds the `.ase` masters from the `*.grid.json` sources, exports atlases, validates them and writes every file marked "pipeline" below into `generated/art/` (git-ignored). The renders are deterministic for the same inputs.

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
```

The scene, close-up and contact-sheet files come from the group render step inside that script, which can also be run on its own after an export:

```bash
python3 scripts/make_scale_comparison.py --group mongo_adult --out generated/art/scale_study/mongo_adult
```

## Files

| File | Content | Origin | Source commit | Regenerate from |
| --- | --- | --- | --- | --- |
| `scene_640x360.png` | Mongo 64, 76 and 88 px, Donut 32 px, Carl 52 px and the 52 px mannequin on one floor line, native 1x | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/scene_640x360.png` |
| `scene_1280x720_2x.png` | the same scene, nearest-neighbour 2x | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/scene_1280x720_2x.png` |
| `scene_1280x800_2x.png` | Steam Deck-class 640x400 framing at 2x | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/scene_1280x800_2x.png` |
| `closeups_side_by_side_6x.png` | 64, 76 and 88 close-ups side by side, nearest-neighbour 6x | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/closeups_side_by_side_6x.png` |
| `closeup_mongo_adult_h64_study_v0_6x_grid.png` | 64 px close-up, 6x with pixel grid | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/closeup_mongo_adult_h64_study_v0_6x_grid.png` |
| `closeup_mongo_adult_h76_study_v0_6x_grid.png` | 76 px close-up, 6x with pixel grid | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/closeup_mongo_adult_h76_study_v0_6x_grid.png` |
| `closeup_mongo_adult_h88_study_v0_6x_grid.png` | 88 px close-up, 6x with pixel grid | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/closeup_mongo_adult_h88_study_v0_6x_grid.png` |
| `comparison_sheet.png` | contact sheet: 1x and 2x scenes plus actor labels (height, width, pivot, colours) | pipeline | `c0bae86` | `generated/art/scale_study/mongo_adult/comparison_sheet.png` |
| `mongo_adult_h76_study_v0-idle.gif` | 76 px four-frame `idle` preview (450/300/550/160 ms), written by `scripts/make_animation_previews.py` | pipeline | `c0bae86` | `generated/art/previews/mongo_adult_h76_study_v0-idle.gif` |
| `idle_h76_3x_dark.gif` | the same four frames at 3x on a dark background | **scratch review render** | grid at `c0bae86` | no committed script; one-off nearest-neighbour render of `assets/source/characters/mongo/mongo_adult_h76.grid.json` frames 0-3 during review |
| `idle_h76_strip_6x.png` | the four idle frames side by side at 6x on a dark background, labelled with frame index and duration, pivot marked (frame 0 to 3, left to right) | **scratch review render** | grid at `c0bae86` | no committed script; same one-off review render as above |

The scratch renders are straight nearest-neighbour enlargements of the committed grid frames with a flat background, plus frame labels and a pivot marker in the strip. They contain no art pixels that are not in the grid source, but they are not produced by a committed script, so they are not reproducible byte for byte from the repository alone.
