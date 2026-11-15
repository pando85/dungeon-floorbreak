# Donut floor 1 scale study: evidence images

Visual evidence for the 32/40/48 px Princess Donut scale study. These are **review aids for a human artistic decision**, not approvals. The evaluation and the proposed (not locked) scale are in [`docs/02-art/donut-scale-study.md`](../../02-art/donut-scale-study.md).

- **Source commit:** `4918b73a4635d4f5ad5d28a02a61e5efca66ea80` (branch `art/donut-floor1-scale-study`). All renders below were produced from the grids and manifests at that commit.
- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; it grants no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` in this directory lists the SHA-256 of every image. `python3 scripts/check_docs.py` verifies it.
- **Not Carl:** the grey 52 px figure in the scene renders is a neutral height mannequin proxy for Carl's provisional 48-56 px height, not Carl.

## Regenerating the pipeline renders

Run from the repository root. `ART_CLI` is any Aseprite or LibreSprite command-line executable. The first command rebuilds the `.ase` masters from the `*.grid.json` sources, exports atlases, validates them and writes every file marked "pipeline" below into `generated/art/` (git-ignored). The renders are deterministic for the same inputs.

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
```

The scene and close-up files come from the group render step inside that script, which can also be run on its own after an export:

```bash
python3 scripts/make_scale_comparison.py --group donut_floor1 --out generated/art/scale_study/donut_floor1
```

## Files

| File | Content | Origin | Regenerate from |
| --- | --- | --- | --- |
| `scene_640x360.png` | 32, 40 and 48 px candidates and the 52 px mannequin on one floor line, native 1x | pipeline | `generated/art/scale_study/donut_floor1/scene_640x360.png` |
| `scene_1280x720_2x.png` | the same scene, nearest-neighbour 2x | pipeline | `generated/art/scale_study/donut_floor1/scene_1280x720_2x.png` |
| `scene_1280x800_2x.png` | Steam Deck-class 640x400 framing at 2x | pipeline | `generated/art/scale_study/donut_floor1/scene_1280x800_2x.png` |
| `closeups_side_by_side_6x.png` | 32, 40 and 48 close-ups side by side, nearest-neighbour 6x | pipeline | `generated/art/scale_study/donut_floor1/closeups_side_by_side_6x.png` |
| `closeup_donut_scale_h32_study_v0_6x_grid.png` | 32 px close-up, 6x with pixel grid | pipeline | `generated/art/scale_study/donut_floor1/closeup_donut_scale_h32_study_v0_6x_grid.png` |
| `closeup_donut_scale_h40_study_v0_6x_grid.png` | 40 px close-up, 6x with pixel grid | pipeline | `generated/art/scale_study/donut_floor1/closeup_donut_scale_h40_study_v0_6x_grid.png` |
| `closeup_donut_scale_h48_study_v0_6x_grid.png` | 48 px close-up, 6x with pixel grid | pipeline | `generated/art/scale_study/donut_floor1/closeup_donut_scale_h48_study_v0_6x_grid.png` |
| `donut_scale_h40_study_v0-idle.gif` | 40 px four-frame `idle` preview (320/260/180/260 ms), written by `scripts/make_animation_previews.py` | pipeline | `generated/art/previews/donut_scale_h40_study_v0-idle.gif` |
| `donut_h40_idle_4x_night.gif` | the same four frames at 4x on a `#191a2c` background | **scratch review render** | no committed script; one-off nearest-neighbour render of `assets/source/characters/donut/donut_h40.grid.json` frames 0-3 during review |
| `donut_h40_idle_strip_6x.png` | the four idle frames side by side at 6x on `#2c2841` (frame 0 to 3, left to right) | **scratch review render** | no committed script; same one-off review render as above |

The scratch renders are straight nearest-neighbour enlargements of the committed grid frames with a flat background. They contain no pixels that are not in the grid source, but they are not produced by a committed script, so they are not reproducible byte for byte from the repository alone.
