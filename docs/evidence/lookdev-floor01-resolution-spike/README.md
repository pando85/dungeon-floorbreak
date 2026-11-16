# Floor 1 look-dev resolution and lighting spike: evidence

Review renders for the A0 / A1 / B1 comparison described in [`docs/02-art/lookdev-floor01-resolution-spike.md`](../../02-art/lookdev-floor01-resolution-spike.md). **Look-dev only.** They are not production art and not approved, and they are not Bevy captures: every file is produced by the Pillow compositor `scripts/lookdev_compose.py`.

- **Authorship and provenance:** authored by Claude Opus (AI art agent) with `scripts/lookdev_floor01_author.py`. The pixel sources are `assets/source/lookdev/floor01/*.grid.json` and `*.ase`. The concept image `assets/concept/floor-01/opening/01-catacomb-environment-clean.png` was used for **mood only** and never sampled. In A0 and A1 the characters are the approved Carl 52 px and Donut 40 px grids, unchanged. In B1 they are the Carl 78 px / Donut 60 px look-dev redraws (frame 0) from branch `art/lookdev-cast-h78-h60`.
- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`rights-and-release.md`](../../04-production/rights-and-release.md). This is an owner-reported publication scope. It is not a reuse licence and gives no right to distribute the game or the assets.
- **Integrity:** `SHA256SUMS` lists every file in this directory; `python3 scripts/check_docs.py` verifies it.
- **Reproducibility:** `python3 scripts/lookdev_compose.py all --out <dir>` rewrites this directory byte for byte. This was checked by rendering twice and comparing SHA-256 sums.

## Files

| Path | Content |
| --- | --- |
| `native/a0_640x360.png`, `native/a0_640x400.png` | A0 at 1×: baked lighting only, 16:9 view and 16:10 view |
| `native/a1_640x360.png`, `native/a1_640x400.png` | A1 at 1×: the same layers with the lighting pass at 640×360 / 640×400 |
| `native/b1_960x540.png` | B1 at 1×: the 960×540 redraw with the lighting pass |
| `4k/a0_3840x2160_x6.png`, `4k/a1_3840x2160_x6.png` | A frames at 3840×2160 (nearest 6× layers; for A1 the light is computed at 4K) |
| `4k/b1_3840x2160_x4.png` | B1 at 3840×2160 (nearest 4× layers, light computed at 4K) |
| `1080p/a*_1920x1080_x3.png`, `1080p/b1_1920x1080_x2.png` | 1920×1080 renders |
| `deck/a0_1280x800_640x400_x2.png`, `deck/a1_1280x800_640x400_x2.png` | Steam Deck: 640×400 view at integer 2× |
| `deck/b1_1280x800_sharp_bilinear_1.33x.png` | Steam Deck: 960×540 at 1.33×, **non-integer**. Nearest 2× prescale, then a bilinear downscale to 1280×720, letterboxed. Labelled in the image. |
| `deck/b1_1280x800_1x_letterbox.png` | Steam Deck: 960×540 at 1×, integer, letterboxed (about 51 % of the screen used). Labelled in the image. |
| `crop_compare_4k_800px.png` | The same 800×800 4K-pixel region around Carl and Donut from A0, A1 and B1, side by side at 1:1. **View it at 100 % on a 4K monitor.** |
| `preview_triptych.png` | Small side-by-side preview (display-downscaled; not for detail judgement) |
| `a1_parallax_demo.gif` | A1 arch region at 2×; gallery layers slide with relative parallax (GIF palette is quantised) |
| `layers/a640_*.png`, `layers/b960_*.png` | Separate native pixel layers, bottom to top, including the actor layer |
| `measurements.json` | Colour counts, actor readability (L*, contrast ratio, silhouette edge ΔL*), opaque pixel area per layer and actor |
