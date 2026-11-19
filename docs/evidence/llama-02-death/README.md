# LLAMA-02-DEATH evidence (design stage and executor pass)

Spec: [llama-02-death-spec.md](../../02-art/llama-02-death-spec.md). All images are stills composed from the committed pixel-grid sources, not Bevy captures, and not a feel approval. The design stage wrote the first group (`python3 scripts/author_llama02_death.py scale`); the executor pass wrote the rest.

**Prerequisite:** the lit scenes need the Floor 1 kit atlases and the two death atlases under `generated/art` (git-ignored). Run `bash scripts/export_assets.sh` once in a fresh checkout. Then regenerate in this order: `python3 scripts/author_llama02_death.py scale`; `python3 -m scripts.pixelart evidence assets/source/characters/llama/death/llama_death_h64.pixelart.json --out docs/evidence/llama-02-death/poses` (and the `parts` config with `--out .../parts`; delete the toolkit's generic `tell_attack_strip.png`, `scene_*.png` and single-frame part GIFs, which are not kept for death sets); `python3 scripts/llama02_death_evidence.py` (also rewrites the per-tag pose GIFs with an exact palette); `python3 -m scripts.pixelart review-pack assets/source/characters/llama/death/llama_death_h64.pixelart.json --out docs/evidence/llama-02-death/review-pack` (keep the three images and `validator_summary.txt`).

| File | What |
| --- | --- |
| `keys_x4_presets.png` | every key at 4x, three rows each: preset 4 (as authored), preset 2 (gore remapped dark), preset 1 (gore hidden, molten kept); bottom strip: every key at 1x, presets 4 and 1 |
| `silhouette_scale_1x.png` | two lit Floor 1 views, 640 x 360 each, 1x, stacked, sprites facing right as authored. Top: Donut (40 px), Carl (52 px), GOB-01 A idle (36 px), LLAMA-01 idle (64 px), `crush_contact` with the ENV-02 `loose_block_falling` drawn over it, `crushed_remains` in front of `loose_block_landed`, `decap_contact`. Bottom: Donut, Carl, `headless_collapse` on lit brick, `slam_contact`, `broken_heap`, and six parts on the walking line |
| `silhouette_scale_4x.png` | the walking-line bands of both views at 4x (nearest neighbour) |
| `scene_640x360.png` | in-scene composition as the runtime draws it (enemies mirrored to face Carl): Donut and Carl unobstructed (drawn last), the crush under the stone drawn over the llama, the remains at the landed stone, the decapitation tick, the headless collapse, the broken heap |
| `scene_1920x1080.png` | the same scene at 3x (1080p) |
| `scene_headless_x3.png` | `headless_collapse` hold on lit brick next to Carl at 3x (gate note N1 recheck) |
| `preset_poses_x4.png` | all eight pose frames at presets 4, 3, 2, 1 (1 hides `gore`, 2 remaps it dark, `molten` always shown; 3 equals 4 for the poses) |
| `preset_parts_x3.png` | all 32 parts at presets 4, 3, 2, 1 (two bands of 16); a part outside the preset's `presets` list is dimmed and crossed out |
| `parts_anchors_x4.png` | the 32 parts at x4 with every anchor marked (white box: cell pivot) |
| `frame_anchor_timing.png` | the eight pose frames with anchors, tags, ticks, authors and `hazard_draw`, plus the crush, decapitation and slam tick timelines |
| `storyboard_crush.png`, `storyboard_decap.png`, `storyboard_slam.png` | the spec section 8 timelines as stills on lit Floor 1 (L, L+3, L+6, L+30; D, D+4, D+10, D+40; S, S+6, S+14, S+40); seeded placement simulation, 90 degree rotation steps; the llama mirrored to face Carl |
| `preset_strip.png` | crush L+30, decapitation D+4 and D+40, slam S+14 at presets 4, 3, 2, 1 |
| `scenes_1x.png` | six 640 x 360 lit views at 1x with Carl and Donut: crush tick, remains with pool stage 3 and the cooled molten puddles (L+250), decapitation tick, headless hold with the head on the floor (D+110), slam heap (S+40), an irregular explosion scatter |
| `scene_carl_in_debris.png` | Carl standing in the debris, pools, molten puddles and wall spatter (cast drawn last: nothing over a player sprite), 1x and a 3x crop |
| `gif_crush_sequence_x1.gif`, `gif_crush_sequence_x3.gif` | poses 0, 1, 2, pool stages 1 to 3 and the molten puddles crusting (L, L+3, L+4, L+6, L+8, L+20, L+60, L+240) |
| `gif_decap_sequence_x1.gif`, `gif_decap_sequence_x3.gif` | D, D+4, D+10, D+14, D+40, D+100: head launch and swap, jet, pools |
| `gif_slam_sequence_x1.gif`, `gif_slam_sequence_x3.gif` | S, S+6, S+8, S+30 (slam pair: unused art until the owner approves the Heavy rule) |
| `poses/`, `parts/` | toolkit contact sheets at x1 and x4 for both atlases; `poses/` also holds the per-tag pose GIFs at x1 and x3 |
| `review-pack/` | exactly three images (contact sheet x4, lit scene, tell and attack strip) plus `validator_summary.txt` |

All GIFs are rendered unlit (or on the flat review background) with an exact palette: one shared palette of every colour used, no quantisation, no dither. A test checks that every frame keeps the wool colour and the blood red.

Checksums: `SHA256SUMS` covers every file in this folder and its subfolders (`poses/` and `parts/` also carry their own list).
