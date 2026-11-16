# Look-dev cast redraws at 1.5x density (Carl h78, Donut h60)

**Status: look-dev only. Not approved art, not a production asset and not a replacement for the approved masters.** These files exist for the virtual-resolution spike: should the game render at 640x360 (current, with the approved Carl 52 px and Donut 40 px masters) or at 960x540? On a 4K display, 640x360 is scaled 6x and 960x540 is scaled 4x, so Carl at 52 px x6 and Carl at 78 px x4 are the same on-screen height (312 px). The redraws let the owner compare how much detail the extra density buys at equal on-screen size.

- **Authored by:** Claude Opus (AI agent), 2026-11-16, as a hand redraw of the approved masters [`carl_h52.grid.json`](../../../assets/source/characters/carl/carl_h52.grid.json) (frame 0) and [`donut_h40.grid.json`](../../../assets/source/characters/donut/donut_h40.grid.json) (frame 0). The masters were not changed.
- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, 2026-11-15, not independently verified; it grants no public reuse licence and no game or standalone asset-pack distribution right).
- **Not in the pipeline:** no manifest under `assets/manifests/` references these sources, so `scripts/export_assets.sh`, the validator and the scale-study groups ignore them.
- **Not used by the ENV-01 kit:** ADR-0007 rejected the 960×540 option; the reworked Floor 1 kit (round 3) and its actor checks use only the approved Carl 52 / Donut 40 atlases. These redraws stay look-dev only.
- **Integrity:** `SHA256SUMS` lists every image in this directory. `python3 scripts/check_docs.py` verifies it.

## Sources

| File | Opaque height | Canvas | Pivot (contact row) | Palette | Frames |
| --- | --- | --- | --- | --- | --- |
| [`carl_h78.grid.json`](../../../assets/source/lookdev/characters/carl_h78.grid.json) + `.ase` | 78 px (rows 6 to 83) | 54 x 84 | [29, 83] | 25 keys, all used | 1 (`idle` 0 to 0, 200 ms) |
| [`donut_h60.grid.json`](../../../assets/source/lookdev/characters/donut_h60.grid.json) + `.ase` | 60 px (rows 6 to 65) | 69 x 66 | [33, 65] | 23 keys, all used | 1 (`idle` 0 to 0, 200 ms) |

- Format `pixel-grid-v1`, same layer names and order as the masters. The `.ase` files were built with `python3 scripts/pixel_grid_to_ase.py build <grid> <ase>`.
- `python3 scripts/pixel_grid_to_ase.py lint <grid> --strict --expect-height 78` (Carl) and `--expect-height 60` (Donut) exit 0: exact height, contact row equals pivot y, 0 orphan and 0 isolated pixels.
- **Pivot convention is the masters' convention**, on the last (contact) row. Carl: midpoint of the two Croc sole runs (x 15 to 26 and 32 to 43, centres 20.5 and 37.5, exact midpoint 29). Donut: centre of the planted-paw span on the contact row (x 12 to 54, centre 33). Both are 1.5x the master pivots (Carl 19 x 1.5 = 28.5, Donut 22 x 1.5 = 33), with headroom scaled the same way (4 master rows, 6 redraw rows).
- **Palette:** the master palettes unchanged, plus intermediate tones. Carl adds `m` skin_warm `#d67d5b` (between `S` and `M`, used for lids, the ear and the jaw) and `t` jacket_fold `#47314f` (between `j` and `T`, used for sleeve and pocket folds). Donut adds `k` cream_mid `#d8b0b6` (between `c` and `C`, a soft step where the chest ruff turns into shadow).
- **Only frame 0.** No 4-frame idle was made. Occluded fills (skin under the jacket, the tail base behind the rump and similar) were **not** redrawn: each visible pixel is assigned to the master layer that holds that colour at the matching position, so the layers are usable for review but not yet for animation.

## Method (honest provenance)

1. The master frame 0 was enlarged 1.5x with nearest-neighbour **as a construction guide only** (an underdrawing for proportions and placement).
2. Every row of both figures was then rewritten by hand in a text grid, region by region (Carl: head, torso and hands, legs and Crocs; Donut: tiara, ears and head, eyes and muzzle, chest ruff and paws, tail and near flank), followed by review renders against the master at equal on-screen size, an orphan pass and two correction passes. Scratch helpers did bookkeeping only (splicing hand-written row segments, compositing, orphan lists, layer assignment); they are not committed.
3. Measured result: 42 % of Carl's and 37 % of Donut's pixels differ from the 1.5x nearest guide. The rest are mostly large flat areas (jacket leather, dark coat, cream ruff) whose colour does not change under a redraw. All jagged 2-1-2-1 stair-steps of the nearest enlargement were removed from outlines and clusters.
4. No pixel was taken from the concept boards, no image model produced any pixel, and there is no smooth or nearest upscale in the final art.

Where the extra resolution went (no new design choices):

- **Carl:** eyes 3 x 2 with almond corners instead of 2 x 1, a low irritated near brow and a raised far brow, a modelled nose (lit ridge and underside, as on the approved 56 px study), a 2-row open "huh?" mouth with lower-lip shade, a 3 x 3 pin, a 2 px watch face, a back-of-hand with knuckle light, thumb and fingertip shadows, 5 x 4 hearts (`RR.RR / RRRRR / .RRR. / ..R..`, three of them, same triangle layout), sleeve elbow crease and pocket fold, shin-hair strokes, Croc hole hints as diagonal highlight pairs and a domed toe.
- **Donut:** 10 x 10 near eye and 9 x 10 far eye with the same lid arc, amber band, 4 x 4 pupil, 2 x 2 catchlight and lower glint; a 3 px nose triangle and a 5 px "w" mouth; a 7 x 7 tiara with a 1 x 3 gem; a 2-row collar with a 5 x 3 butterfly charm; the same tortoiseshell mask, blaze, cheek patches and flank blotches; ruff tuft clusters as `W` pairs over `c` "v" scallops; tail fringe as `G`/`O` tufts with `o` notches and a `W` tip; three planted paws with toe lines.

## Self-review (4x, against the master at equal height)

Identity check: both read as "the same Carl / the same Donut, with more detail". Pose, silhouette, proportions, costume, coat map and palette families match the masters. First-pass problems that were fixed: Carl's hair read lighter and stripier than the master (dashes reduced, darker mass), his first face pass had a heavy cheek band and a vertical nose ridge that read as a big nose (removed), the fingertips ended in a black bar (opened with boxer white as on the master), and the Croc toes were one pixel too long. Donut's tiara gem read as a cross (now a vertical 1 x 3 bar as on the master), a seam left a transparent column beside the ruff (filled), and the upper tail core was darker than the master's marbling (more ginger).

Known weaknesses: Carl's chest carries more shading marks than the master; the near hand is a little blockier than a hand-drawn 78 px hand would ideally be. Donut's far cheek has a dark notch above it that is slightly heavier than on the master, and her chest ruff is still a large cream area. Neither redraw has been seen in the Bevy runtime.

## Files

All images are deterministic: the 1x PNGs are LibreSprite exports of the committed `.ase` files, and the comparison sheets are nearest-neighbour Pillow renders of frame 0 of the committed grids (cropped to the opaque bounding box, bottom-aligned, labels in Pillow's default bitmap font). The sheet script is scratch tooling and is not committed; it also asserted that each LibreSprite export is pixel-identical to the grid.

| File | Content |
| --- | --- |
| `carl_h78_idle_f0_1x.png` | Carl h78 frame 0, transparent, 1x (54 x 84), exported with `libresprite -b assets/source/lookdev/characters/carl_h78.ase --save-as <png>` |
| `donut_h60_idle_f0_1x.png` | Donut h60 frame 0, transparent, 1x (69 x 66), same command with `donut_h60.ase` |
| `compare_carl_h52x3_vs_h78x2.png` | approved h52 nearest x3 next to look-dev h78 nearest x2, both 156 px tall |
| `compare_donut_h40x3_vs_h60x2.png` | approved h40 nearest x3 next to look-dev h60 nearest x2, both 120 px tall |
| `compare_carl_h52x6_vs_h78x4.png` | 4K integer scales: h52 x6 (640x360) next to h78 x4 (960x540), both 312 px tall |
| `compare_donut_h40x6_vs_h60x4.png` | 4K integer scales: h40 x6 next to h60 x4, both 240 px tall |
