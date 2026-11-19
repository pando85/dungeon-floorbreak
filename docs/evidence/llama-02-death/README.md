# LLAMA-02-DEATH evidence (design stage)

Spec: [llama-02-death-spec.md](../../02-art/llama-02-death-spec.md). All images are stills composed from the committed pixel-grid sources, not Bevy captures, and not a feel approval.

**Prerequisite:** the lit scenes need the Floor 1 kit atlases under `generated/art` (git-ignored). Run `bash scripts/export_assets.sh` once in a fresh checkout. Then `python3 scripts/author_llama02_death.py scale` regenerates the top-level stills, and `python3 -m scripts.pixelart evidence assets/source/characters/llama/death/llama_death_h64.pixelart.json --out docs/evidence/llama-02-death/poses` (and the `parts` config with `--out .../parts`) the toolkit output; the toolkit's generic `tell_attack_strip.png` and `scene_*.png` are not kept for death sets, nor single-frame part GIFs.

| File | What |
| --- | --- |
| `keys_x4_presets.png` | every key at 4x, three rows each: preset 4 (as authored), preset 2 (gore remapped dark), preset 1 (gore hidden, molten kept); bottom strip: every key at 1x, presets 4 and 1 |
| `silhouette_scale_1x.png` | two lit Floor 1 views, 640 x 360 each, 1x, stacked, sprites facing right as authored. Top: Donut (40 px), Carl (52 px), GOB-01 A idle (36 px), LLAMA-01 idle (64 px), `crush_contact` with the ENV-02 `loose_block_falling` drawn over it, `crushed_remains` in front of `loose_block_landed`, `decap_contact`. Bottom: Donut, Carl, `headless_collapse`, `slam_contact`, `broken_heap`, the part keys `head`, `neck`, `leg_fore` on the walking line |
| `silhouette_scale_4x.png` | the walking-line bands of both views at 4x (nearest neighbour) |
| `scene_640x360.png` | in-scene composition as the runtime draws it (enemies mirrored to face Carl): Donut and Carl unobstructed, the crush under the stone, the remains at the landed stone, the decapitation tick, the broken heap |
| `scene_1920x1080.png` | the same scene at 3x (1080p) |
| `poses/`, `parts/` | toolkit output: contact sheets at x1 and x4, pose GIFs at x1 and x3 (frames 1 and 4 are placeholder copies until the executor pass) |

Checksums: `SHA256SUMS` covers every file in this folder and its subfolders (`poses/` and `parts/` also carry their own list).
