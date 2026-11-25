# CARL-GAUNTLET-01 evidence: key frames and silhouette gate

Key-frame previews for [the CARL-GAUNTLET-01 spec](../../02-art/carl-gauntlet-01-spec.md), rendered nearest-neighbour from the committed key file `assets/source/characters/carl/gauntlet_keys/carl_h52_gauntlet_keys.grid.json`. They show the three frozen keys only. The 98 overlay frames, contact sheets, timing GIFs and in-room captures belong to the execution step.

| File | Content |
| --- | --- |
| `keys_bare_vs_gauntlet_x8.png` | Three pairs at x8 on a flat `#342c46` background, bare then gauntlet: `idle_hand` (locomotion 0), `punch_b_windup` (melee 4), `punch_b_contact` (melee 5) |
| `keys_hand_zoom_x20.png` | The same three pairs cropped to a 16 x 14 window around the hand, x20 |
| `gate_floor1_640x360_x1.png` (640 x 360), `gate_floor1_1920x1080_x3.png` (1920 x 1080) | Lit Floor 1 composition at gameplay scale, left to right: bare idle, gauntlet idle, gauntlet windup, gauntlet contact, bare contact, gauntlet idle mirrored (the runtime `flip_x`) |

The scenes use the toolkit's Floor 1 layout and lighting pass (`scripts.pixelart.scene.Floor`, layout `assets/levels/floor01/corridor_abc_sample.json`, view `abc_640x360`); the kit atlases come from `bash scripts/export_assets.sh`. The gauntlet layer is composed on top of the body layers, as the runtime overlay will be.

**Gate:** PASS with notes (spec section 10). The notes (a brown leather wrist strap, a notch in each steel highlight, the brightest steel on the striking corner) are folded into the keys shown here.

**Provenance:** the gauntlet pixels are hand-placed on the approved ANIM-CARL-02 body frames, which are copied unchanged; no concept image, third-party design or image-model raster output is used, copied, traced or resampled. Published under the owner-reported scope `ART-PUBLICATION-ALL-V1` ([rights register](../../04-production/rights-and-release.md)); no public reuse licence and no distribution rights for a game or asset pack.
