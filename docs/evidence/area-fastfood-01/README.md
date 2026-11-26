# AREA-FASTFOOD-01: the fast-food refuge kit (room 1)

Evidence for [the AREA-FASTFOOD-01 spec](../../02-art/area-fastfood-01-spec.md): the kit drawn and wired into room 1 `rest_refuge` ("The Wrong Restaurant"), the real captures, the offline sheets and the measurements, plus the silhouette gate sheets of the design step (`gate/`). Everything here is prototype art (`prototype_fixture_unapproved`): the independent review is pending, and nothing is owner-approved. Owner art review pending; game feel is the owner's call.

## Real Bevy captures (`scene_*`)

Each still is the REST-WIRE capture command rendered in a real window under xvfb, in three sizes: `640x360` (`--scale 1`), `1920x1080` (`--scale 3`) and the `640x400` deck view (`--deck --scale 1`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S [--deck] \
  --scene f01_late --f01-room rest_refuge --rest-pilot donut|carl_door --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`scripts/make_ff01_evidence.sh` captured every still **twice** (two full passes): the two PNGs are byte-identical, and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`) and, where REST-WIRE captured the same pilot, tick and size, equals the REST-WIRE capture's hash (`rest_wire_match=yes`): the art is presentation only. [`capture_lines.txt`](capture_lines.txt) holds one `FF_STILL` line per still and size. The drawn view (left, top) of each 640 x 360 still is measured from landmarks by a 2D match against the lit model (`scripts/ff01_view.py`; the logged `cam` is not the drawn view) and is in [`drawn_views.txt`](drawn_views.txt).

| Still | Pilot, tick | Drawn view left | What it shows |
| --- | --- | --- | --- |
| `threshold` | `donut`, 20 | 0 | the hot side: the catacomb, the two threshold torches, the timber door, the seam where the tile wall begins (F-A0) |
| `play_area` | `donut`, 120 | 130 | the deck and slide-top skins, the posts and net, the pit, the chute, the counter and its canopy, the bin, the light strips |
| `window` | `donut`, 270 | 620 | bay B as a restaurant window onto raw rock, the booth, the stray ball |
| `box_full` | `donut`, 312 | 757 | the full box, Donut approaching |
| `box_grant` | `donut`, 332 | 822 | the grant: the box already spent (empty iff the boost is held) with sparkle frame 2 over it |
| `door_closed` | `carl_door`, 560 | 960 | the closed staff door, the service hatch, the ledge, the `SWAP TO DONUT` notice. The box is spent: the companion crossed it on the way (the gate composite's "box untouched" was a design-stage assumption) |
| `door_rising_371`, `_373`, `_376` | `donut`, 371, 373, 376 | 944, 951, 960 | rising frames 2, 4 and 7 of the 12, the bell ringing, Donut on the ledge |
| `door_open` | `donut`, 392 | 960 | the open door (rails only), the bell idle at the ledge's end |
| `exit` | `donut`, 408 | 960 | the vestibule: back to catacomb masonry and the closed portal (F-A4) |

The lit scene is darker and cooler than the key sheets: the **full box and the spent box both read as dull olive at 1x** under the room's cool ambient (the box has no emitter, FF-AD6, FF-AD23). This is a note for the owner's art review (FF-A10), not a tweak.

## Offline sheets

| File | Content |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | every piece of the kit at native size and labelled at 4x (the rising door crops are in the door strip) |
| `door_states_x4.png` | the 14 door frames at 4x: closed, rising 1..12, open |
| `box_states_x4.png` | the box's three frames, the sparkle over the empty box, and the bell's three frames at 4x |
| `layout_diagram_play_x2.png`, `layout_diagram_shelf_x2.png` | the schema at 2x over the real `play_area` and `door_open` stills: the walking line W 580, the floor back line FB 560, the three stand lines with their 64 px envelopes, the lock rect, the grant column, the presenter rects and every dressing rect |
| `before_after_door_closed_x1.png`, `_x3.png` | the REST-WIRE greybox still of the same pilot and tick beside the dressed room (`carl_door`, 560) |
| `before_after_shelf_376_x1.png`, `_x3.png` | the same for `donut`, 376 (the door is rising 7 of 12 in the dressed room) |
| `measurements.txt` | gate note G7, rule 10.1 on the runtime stills, the light count, the colour budgets and the key pins |

## Measurements (`measurements.txt`)

- **G7 (door against wall):** in the runtime `door_closed` still the door panel's interior (x 1441..1454, y 368..527) has mean luma 47.8 and the wall field beside it (x 1408..1435) 72.3: a difference of **24.5, a miss of the target 25 by 0.5**. The rail's `stone_cold` 0 outline is unbroken (darker than both neighbours on all 160 rows). Reported as a design note, not tweaked.
- **Rule 10.1 (G2 metric) on the runtime stills:** each stand line is the brightest 8 px run in its 64 px envelope in every still where it is visible (the lit model explains the room, the actors, HUD and presenter art are masked, the three stand-line pieces are not counted as rivals). The unit test `tests/test_art_ff01.py` proves the same on the pixel model.
- **Lights:** the worst 640 px view holds 28 lights (the greybox's: 31) against `MAX_LIGHTS` 64; the net change is minus 8 plus 6 (`fastfood_kit` test).

## Regenerating the gate sheets

`python3 scripts/author_ff01.py gate docs/evidence/area-fastfood-01/gate` regenerates 14 of the 15 gate files and they stay **byte-identical** while the keys are unchanged (the capture script regenerates them). **`gate/gate_overview.png` is assembled from the gate sheets by hand and is not regenerated by the `gate` command** (gate note G6); its committed bytes and its `SHA256SUMS` line are kept. The gate's `keys_in_*` and `states_in_scene_*` images are gate composites, not runtime captures.

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
