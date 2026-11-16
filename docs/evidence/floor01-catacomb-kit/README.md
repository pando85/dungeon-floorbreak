# Floor 01 catacomb environment kit (ENV-01), rework round 4: evidence images

These are visual review aids for the ENV-01 kit described in [`docs/02-art/floor-01-catacomb-kit.md`](../../02-art/floor-01-catacomb-kit.md), reworked against the owner-approved A1 target frame ([ADR-0007](../../05-decisions/ADR-0007-rendering-resolution-lighting.md)). They are **art previews composited by scripts from the exported atlases and the sample layout**. They are **not Bevy screenshots**, not collision or gameplay proof, and not approvals. Owner-approved on 2026-11-16 (PR #26); Bevy lighting re-tune pending in COMP-01. In-engine evidence is the job of COMP-01. The round-1/2 images were rejected and removed; round-3 images were replaced by round 4 (independent review fixes).

- **Source:**
  - branch `art/env-01-floor01-catacomb-kit`;
  - kit contract `assets/source/terrain/floor01/floor01_catacomb_kit.json` (`env-kit-v2`);
  - sample layout `assets/levels/floor01/corridor_abc_sample.json` (`env-kit-layout-v2`).
- **Renderers:**
  - `unlit_*`: `scripts/env_kit.py all` (run by `scripts/export_assets.sh`). It only pastes rects from `generated/art/floor01_*.png` in the contract's draw order; parallax strips are placed for the view's camera.
  - `lit_*`, `actor_check_lit_x4.png`, `measurements.json`: `scripts/env_kit_light.py`. It adds the presentation-only A1 lighting model (cold ambient per light group, warm emitter pools, floor pools on the walking line, gallery haze, glow, flame bloom, actor contact shadow) at an integer output scale. Pixel layers are only ever nearest-upscaled. A second run is byte-identical.
- **Integrity:** `SHA256SUMS` lists every file in this directory, and `python3 scripts/check_docs.py` verifies it. The images were rendered locally with Pillow 12.3 and a distribution LibreSprite build; CI re-exports the atlases from the sources. PNG bytes may differ across Pillow versions even when pixels are identical.
- **Rights:**
  - The environment art is original agent-authored pixel art (Claude Opus).
  - The right panel of the side-by-sides is the A1 look-dev frame `docs/evidence/lookdev-floor01-resolution-spike/native/a1_640x360.png` (same project, same agent, owner-approved target), nearest-scaled for display.
  - The actor images composite the approved Carl 52 px and Donut 40 px atlases (frame 0) under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence, and grants no game or asset-pack distribution right. The h78/h60 look-dev redraws are not used.

## Regenerating

```bash
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/env_kit_light.py --out docs/evidence/floor01-catacomb-kit
# unlit_* copies: generated/art/env_kit/floor01_corridor_abc_sample/<name without the unlit_ prefix>
# (unlit_vs_a1_target_side_by_side.png <- corridor_unlit_vs_a1_target.png)
# floor01_torch_v2-burn.gif <- generated/art/previews/floor01_torch_v2-burn.gif
```

## Files

| File | Content |
| --- | --- |
| `lit_abc_1920x1080_x3.png` | **Primary display check:** view `abc_640x360` (A → B → C, same composition as A1), lit, with Carl 52 and Donut 40 at the A1 positions, 1080p = nearest 3× |
| `lit_abc_640x360.png` | The same frame at native 640×360 |
| `lit_abc_1280x800_640x400_x2.png` | Steam Deck framing: view `abc_640x400`, lit, nearest 2× |
| `lit_abc_env_only_640x360.png` | The lit frame without actors |
| `lit_overview_2048x400.png` | The whole 128×25 sample layout, lit (parallax camera at x 0), with all four actor-check placements (two on platforms) |
| `lit_vs_a1_target_side_by_side.png` | Labelled: left the lit kit frame, right the A1 target frame, both nearest 2× |
| `actor_check_lit_x4.png` | Readability close-up of Carl 52 / Donut 40 on the walking line (0 px sink), lit, nearest 4× crop |
| `measurements.json` | Readability (mean ΔL*, contrast ratio, silhouette edge \|ΔL*\|, edge by height band) for the lit and unlit frames and the A1 reference; colour counts per atlas and of the lit frame; SHA-256 of each lit file |
| `unlit_corridor_abc_640x360.png` | The same view with pixel layers only (baked light, no lighting pass) |
| `unlit_corridor_overview_2048x400.png` | Whole layout, pixel layers only |
| `unlit_actor_check_640x360.png` | Pixel layers only, with Carl and Donut |
| `unlit_vs_a1_target_side_by_side.png` | Unlit kit view (2×) next to the lit A1 frame, to show how much comes from baked pixels |
| `unlit_kit_sheet_1x.png`, `unlit_kit_sheet_2x.png` | Exploded contact sheet of every tile, piece and torch frame, 1× and nearest 2× |
| `floor01_torch_v2-burn.gif` | The 4-frame torch loop (120/140/100/130 ms), written by `scripts/make_animation_previews.py` |
