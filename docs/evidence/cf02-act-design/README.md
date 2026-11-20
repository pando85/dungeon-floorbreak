# CF02 ACT-DESIGN evidence: acting key poses

Key-pose previews for [the CF02 acting spec](../../02-art/cf02-acting-spec.md), rendered nearest-neighbour from the committed `cf02_keys` grids. They show the keys only: the derived frames, contact sheets of the full sheets, timing GIFs and in-scene GIFs belong to the execution PRs (ART-CARL-CF02, ART-DONUT-CF02, ART-GOB-CF02), which regenerate their evidence with the pixel-art toolkit.

**Note:** the `smug` key shown in `donut_keys_1x.png` and `donut_keys_3x.png` predates the ART-DONUT-CF02 review amendment (10 px neck and chin closure); the amended key is shown in [`docs/evidence/cf02-donut/`](../cf02-donut/README.md).

| File | Content |
| --- | --- |
| `carl_keys_1x.png`, `carl_keys_3x.png` | Carl: `stomp_knee_up`, `stomp_stamp`, `dive_heavy_landing`, `flinch`, `frustration_shake`, `air_kick_boot`, `dive_tuck`, `dive_plunge`, `bailout_scramble` on a flat `#2a2a3a` background (3x: labels, frame border, red pivot pixel, floor line) |
| `donut_keys_1x.png`, `donut_keys_3x.png` | Donut: `startle_puff`, `smug` |
| `goblin_keys_1x.png`, `goblin_keys_3x.png` | Goblin A: `hesitate`, `panic_flee`, `taunt_thump`, `stomped_impact`, `stomped_rest`, `wall_impact`, `wall_rest`, `flung_impact`, `flung_rest` |
| `scene_readability_1x.png` (640 x 360), `scene_readability_3x.png` (1920 x 1080) | Lit Floor 1 composition at gameplay scale: a stamp on a stomped goblin, a jump-in air kick on a taunting goblin, `wall_impact` and `wall_rest` each against a wall stub on their contact column, `dive_tuck` and `dive_plunge` in the air, a flung face-plant and heap, the heavy landing, a bail-out hop next to a fleeing goblin, the flinch and the knee-up. Goblins face Carl (mirrored, the runtime rule) except the fleeing one, which faces its flee direction. Airborne keys are lifted off the walking line by hand |
| `scene_acting_1x.png`, `scene_acting_3x.png` | Lit Floor 1 composition of the acting overlays: Carl's frustration, Donut smug and startled, goblin idle 0-3 with `hesitate` among them, panic and taunt |
| `hesitate_check_1x.png`, `hesitate_check_3x.png` | The unlabelled hesitate check of spec section 5: goblin idle frames 1, 3, 0, 2 and `hesitate` in a shuffled order |

The lit scenes use the toolkit's Floor 1 layout and A1 lighting pass (`scripts.pixelart.scene.Floor`, view `abc_640x360`); the kit atlases come from `bash scripts/export_assets.sh`. The wall stubs are plain placeholder rectangles, not kit tiles.

Readability notes at 1x: the stomp, air kick, dive pair, panic and the three slammed rests read; the hesitate is picked out of the idle strip by its turned head, raised hand and club held upright against the body; `wall_rest` reads as dead (lolled head, flat legs, smear) rather than getting up; the bail-out reads as an alarmed backpedal (yelling head, one arm flailing up, one hand warding).

Review fix (ART-CARL-CF02, PR #146): three 1 px pinholes were closed in the key files after the independent review (`stomp_knee_up` jacket (21, 19), `dive_plunge` far_arm (30, 15), `bailout_scramble` jacket (28, 30)); the renders in this folder are the historical design-phase renders.
