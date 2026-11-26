# AREA-HEAT-01: the scorched corral kit (room 0)

Evidence for [the AREA-HEAT-01 spec](../../02-art/area-heat-01-spec.md). This stage holds only the design keys and the silhouette gate sheets, regenerated after the first gate: the partition carries K2 and K3, and `heat_scorch_wall_a` is the R1..R8 redraw (spec section 11.2, `HEAT-AD11`). The rest of the kit, the room dressing, the Bevy wiring and the real in-scene captures follow in execution (spec section 13). Prototype art: nothing here is owner-approved. Game feel is the owner's call.

## Silhouette gate (`gate/`)

| File | Content |
| --- | --- |
| `keys_1x.png`, `keys_4x.png` | the five keys at native size and at 4x on a neutral ground: `heat_partition_intact`, `_weakened`, `_broken`, `heat_scorch_wall_a`, `heat_scorch_floor_a` |
| `silhouettes_1x.png`, `silhouettes_4x.png` | the same keys as flat silhouettes (alpha only) |
| `scene_pen_before_1x.png` | **real capture:** the corral at tick 510 of `scripts/room_capture.sh corral corral 420:30:7` (640 x 360, `--scale 1`), Carl, Donut and llama B in the pen; window hash equals headless hash (`a43c05ca68a91c2e`, `match=yes`). The background of every composite below |
| `scene_pen_intact_1x.png`, `_2x.png` | **gate composite (CORRAL-D pending), not runtime:** the real still (drawn view x 1038, y 289) with the intact partition at (1380, 532), `heat_scorch_wall_a` at (1088, 464), `heat_scorch_floor_a` at (1044, 561) and a goblin, drawn in the kit's layer order (the entry shelf draws over the wall decal, the actors over props and decals). The wall decal is lit by an approximation: each of its bricks is scaled per channel by the light the still shows on that same brick |
| `scene_pen_weakened_1x.png`, `_2x.png` | **gate composite (CORRAL-D pending), not runtime:** the same with the weakened partition |
| `scene_pen_broken_1x.png`, `_2x.png` | **gate composite (CORRAL-D pending), not runtime:** the same with the broken remnant |
| `states_in_scene_1x.png`, `_4x.png` | **gate composite (CORRAL-D pending), not runtime:** the three partition states side by side in the pen crop, next to llama B |
| `scorch_wall_before_after_1x.png`, `_3x.png` | **composite, not runtime:** the wall scorch before (left) and after (right) at gameplay scale, crop x 10..149, y 150..289 of the still: no actor (the "not a door" check, R7), the goblin at its gate position, the goblin over the plume (x 72), Carl at the pen entry (x 72) |
| `measurements.txt` | the key pins (pixel-grid SHA-256), sizes, colour counts, the partition's mean and cap L* against what is behind it, the deck's p95 L*, the wall scorch's R2..R6 checks (foot and widest row, edge runs, holes, brick indices per band, base-course content, embers) and the actors' ΔL* against their 6 px ring with and without the scorch, and the floor scorch coverage |

**Staged:** the goblin is frame 0 of the exported `goblin_a_h36` atlas, pasted at layout x 1078 on the walking line (and at x 1110, over the plume, in the before/after sheet); Carl is frame 0 of `carl_book1_h48` at x 1110. M0's goblins (x 336, 416) never reach the pen. The partition does not exist in the room yet (CORRAL-D, spec `HEAT-AD1`), so no runtime still can show it.

**Not run in this stage:** every in-scene capture of the dressed room and every lit runtime measurement (C5), 640 x 400 and 1920 x 1080, contact sheets of the full kit, export of the kit atlases, the dressing tests (execution).
