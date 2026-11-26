# AREA-FASTFOOD-01: the fast-food refuge kit (room 1), silhouette gate

Evidence for [the AREA-FASTFOOD-01 spec](../../02-art/area-fastfood-01-spec.md). This is the design step only: the eight keys and the silhouette gate. The kit, the room dressing, the presenter and the Bevy wiring are not executed. Everything here is prototype art (`prototype_fixture_unapproved`). It is not reviewed and not owner-approved. **Every `keys_in_*` and `states_in_scene_*` image is a gate composite, not a runtime capture.**

Regenerate the sheets with `python3 scripts/author_ff01.py gate docs/evidence/area-fastfood-01/gate`. This reads the two committed `scene_*` stills.

## Scene stills (`gate/scene_*_1x.png`)

Two real window frames of the merged REST-WIRE greybox at 640 x 360 (`--scale 1`), taken under xvfb. Both stills use the same view: left x 960, top y 292, checked against the shelf plank and Donut's feet on y 451. Command:

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale 1 \
  --scene f01_late --f01-room rest_refuge --rest-pilot PILOT --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

The stills were taken on a local build whose greybox decor rectangles and door rectangle were switched off: the early return at the top of `spawn_rest_greybox`, an uncommitted capture-only edit. This gives the keys a plain room to sit in. The rectangles are presentation only. The window `COMBAT_STATE` hashes equal the REST-WIRE captures' at the same ticks: `7de8e56a1dd76c57` (tick 560) and `68f705ad30d7c97b` (tick 376).

| Still | Pilot, tick | State |
| --- | --- | --- |
| `scene_carl_closed_1x.png` | `carl_door`, 560 | lock closed, box untouched, solo Carl at the door (`SWAP TO DONUT`) |
| `scene_donut_open_1x.png` | `donut`, 376 | lock open (event tick 370), box spent (grant tick 326), Donut on the shelf |

## Gate sheets (`gate/`)

| File | Content |
| --- | --- |
| `keys_1x.png`, `keys_4x.png` | the eight keys at native size and at 4x: door closed and open, box `full_a`, `full_b` and `empty`, ledge, wainscot, checker |
| `silhouettes_1x.png`, `silhouettes_4x.png` | the same keys as flat silhouettes on a light ground |
| `door_rising_1x.png`, `_2x.png` | closed, then the crops of rising ticks 1, 3, 5, 7 and 9 of spec section 6, then open |
| `keys_in_carl_closed_1x.png`, `_2x.png` | gate composite: closed door, full box, the ledge skin, wainscot and checker samples on the staff wall, in the real still. Carl and Donut are re-pasted on top (actor pixels found by differencing the two stills inside the actor boxes) |
| `keys_in_donut_open_1x.png`, `_2x.png` | gate composite: open door, spent box, Donut on the ledge, Carl at the box |
| `states_in_scene_1x.png`, `_4x.png` | the two composites cropped to world x 1180..1470, y 340..584 side by side: the door and box state pairs at gameplay scale |
| `gate_overview.png` | both 2x composites, the rising strip and the 4x keys on one sheet |
| `measurements.txt` | door coverage, the open lane, colour counts, luma of the box frames and the stand line, and the SHA-256 pins of the eight keys |

## Readings

- Closed door: 3648 of 3648 solid px are covered. Open door: 0 of 3552 px are opaque in lane rows 4..225. Only the 4-row lip under the slab and the 2-row sill track remain.
- The box's mean luma is 137 full and 85 spent. The spent box has no glint.
- The ledge's brightest row is row 1, y 449 (`flame` 5, luma 250): the kit plank's own lit row. Rows 0..15 are opaque, so the plank tile is covered.
- The samples stay under the stand line: wainscot max luma 135, checker max 201.
