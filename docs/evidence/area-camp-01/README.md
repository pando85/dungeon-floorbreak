# AREA-CAMP-01 evidence

**Status:** silhouette gate material only. The design keys are prototype art, the independent gate is pending and nothing is owner-approved. The kit is not executed yet: no contract, no dressing and no Bevy wiring exist. Spec: [`docs/02-art/area-camp-01-spec.md`](../../02-art/area-camp-01-spec.md).

## `gate/`

Written by `python3 scripts/author_camp01.py gate CAPS gate/ [BOARD]`.

| File | Shows |
| --- | --- |
| `keys_1x.png`, `keys_4x.png` | the twelve key frames: hearth, fire 0..3, sentry, wreck, bag rest/r1/r2/l1/l2 |
| `silhouettes_1x.png`, `silhouettes_4x.png` | the same frames as flat silhouettes |
| `fire_loop_4x.png` | the `camp_fire` loop |
| `bag_sway_1x.png`, `bag_sway_3x.png` | the bag's five frames |
| `keys_in_camp_entrance_*.png` | `rest_camp`, tick 41: the marker, the lean-to and the sentry |
| `keys_in_camp_hearth_*.png` | `rest_camp`, tick 281: the crates and the hearth with fire frame 1 |
| `keys_in_aft_wreck_*.png` | `rest_aftermath`, tick 161: the wreck and the bag at `r2` |
| `keys_in_aft_lane_*.png` | `rest_aftermath`, tick 281: the bag at rest and the lane |
| `gate_overview.png` | the four scenes at 1x and the keys at 2x on one board |
| `measurements.txt` | sizes, colour counts, luma checks, torch-column clearance, the sway table and the SHA-256 pins of the key frames |

**How the scenes are made.** The scene stills are 640 x 360 window frames from the walk pilot (`--scene f01_late --f01-room ROOM --rest-pilot walk --scale 1`). They come from a scratch probe build that was not committed:
- the greybox decor rectangles are switched off;
- the reused rings and gym pieces of spec section 5 are applied through a scratch dressing, with a light-only stand-in at the fire's position;
- each tick is captured twice, with and without the actor sprites.

The keys are composed at 1x, unlit, over the actor-free frame. Every pixel where the two frames differ is re-pasted from the actor frame. The view left of each still is measured from the torch flames: tick 41 at 0, tick 161 at 237, tick 281 at 607. These scenes are a design preview, not runtime evidence; the execution replaces them with real captures whose window hash equals the headless hash.
