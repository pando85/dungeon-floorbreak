# Carl Book 1 scale study: evidence images

Visual evidence for the 48/52/56 px Carl "Survivor" (Book 1) native pixel scale study and the 52 px idle prototype. These are **review aids for a human artistic decision**, not approvals. The evaluation and the proposed (not locked) scale are in [`docs/02-art/carl-scale-study.md`](../../02-art/carl-scale-study.md).

- **Source commit:** `957270e36ecf9fccdaca2e2bb006dfcb3d03d7c8` (branch `art/carl-book1-sprite-scale`, stacked on `art/donut-floor1-scale-study`). All pipeline renders below were produced from the grids and manifests at that commit. The Donut 40 px companion in the scene comes from the Donut study commit `4918b73`.
- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; it grants no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` in this directory lists the SHA-256 of every image. `python3 scripts/check_docs.py` verifies it.
- **Not Carl:** the grey 52 px figure in the scene renders is a neutral height mannequin proxy for Carl's provisional 48-56 px height, not Carl. Left to right in the scene: mannequin, Donut 40 px companion, Carl 48, 52, 56.

## Regenerating the pipeline renders

Run from the repository root. `ART_CLI` is any Aseprite or LibreSprite command-line executable. The first command rebuilds the `.ase` masters from the `*.grid.json` sources, exports atlases, validates them and writes every file marked "pipeline" below into `generated/art/` (git-ignored). The renders are deterministic for the same inputs.

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
```

The scene, close-up and contact-sheet files come from the group render step inside that script, which can also be run on its own after an export:

```bash
python3 scripts/make_scale_comparison.py --group carl_book1 --out generated/art/scale_study/carl_book1
```

## Files

| File | Content | Origin | Source commit | Regenerate from |
| --- | --- | --- | --- | --- |
| `scene_640x360.png` | Carl 48, 52 and 56 px, the Donut 40 px companion and the 52 px mannequin on one floor line, native 1x | pipeline | `957270e` | `generated/art/scale_study/carl_book1/scene_640x360.png` |
| `scene_1280x720_2x.png` | the same scene, nearest-neighbour 2x | pipeline | `957270e` | `generated/art/scale_study/carl_book1/scene_1280x720_2x.png` |
| `scene_1280x800_2x.png` | Steam Deck-class 640x400 framing at 2x | pipeline | `957270e` | `generated/art/scale_study/carl_book1/scene_1280x800_2x.png` |
| `closeups_side_by_side_6x.png` | 48, 52 and 56 close-ups side by side, nearest-neighbour 6x | pipeline | `957270e` | `generated/art/scale_study/carl_book1/closeups_side_by_side_6x.png` |
| `closeup_carl_book1_h48_study_v0_6x_grid.png` | 48 px close-up, 6x with pixel grid | pipeline | `957270e` | `generated/art/scale_study/carl_book1/closeup_carl_book1_h48_study_v0_6x_grid.png` |
| `closeup_carl_book1_h52_study_v0_6x_grid.png` | 52 px close-up, 6x with pixel grid | pipeline | `957270e` | `generated/art/scale_study/carl_book1/closeup_carl_book1_h52_study_v0_6x_grid.png` |
| `closeup_carl_book1_h56_study_v0_6x_grid.png` | 56 px close-up, 6x with pixel grid | pipeline | `957270e` | `generated/art/scale_study/carl_book1/closeup_carl_book1_h56_study_v0_6x_grid.png` |
| `comparison_sheet.png` | contact sheet: 1x and 2x scenes plus actor labels (height, width, pivot, colours) | pipeline | `957270e` | `generated/art/scale_study/carl_book1/comparison_sheet.png` |
| `carl_book1_h52_study_v0-idle.gif` | 52 px four-frame `idle` preview (420/260/560/260 ms), written by `scripts/make_animation_previews.py` | pipeline | `957270e` | `generated/art/previews/carl_book1_h52_study_v0-idle.gif` |
| `carl_h52_idle_x4_2c2841.gif` | the same four frames at 4x on a `#2c2841` background | **scratch review render** | grid at `957270e` | no committed script; one-off nearest-neighbour render of `assets/source/characters/carl/carl_h52.grid.json` frames 0-3 during review |
| `carl_h52_idle_strip_x6.png` | the four idle frames side by side at 6x on `#2c2841` (frame 0 to 3, left to right) | **scratch review render** | grid at `957270e` | no committed script; same one-off review render as above |

The scratch renders are straight nearest-neighbour enlargements of the committed grid frames with a flat background. They contain no pixels that are not in the grid source, but they are not produced by a committed script, so they are not reproducible byte for byte from the repository alone.
