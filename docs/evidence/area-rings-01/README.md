# AREA-RINGS-01: the Broken Ring kit (rooms 4a and 4b)

Evidence for [the AREA-RINGS-01 spec](../../02-art/area-rings-01-spec.md): the kit drawn and wired into both rooms, the stone shutter presenter, the real captures and the measurements. Prototype art (`prototype_fixture_unapproved`): the silhouette gate passed with the key edits K1..K4 (spec section 16), the independent review is pending, and nothing here is owner-approved. Game feel is the owner's call.

## Real Bevy captures (`scene_*`)

Every still is the real window of `--scene f01_late --f01-run --f01-room ROOM --ring-pilot`, under xvfb, in three sizes: `640x360` (`--scale 1`), `1920x1080` (`--scale 3`) and the `640x400` deck view (`--deck --scale 1`). `scripts/make_rings01_evidence.sh` captured each one **twice** (two full passes) and the two PNGs are byte-identical; the window's final `COMBAT_STATE` hash equals the headless run's at the same tick for every line. [`capture_lines.txt`](capture_lines.txt) holds one `RINGS_STILL` line per still and size (33 lines, 11 stills in 3 sizes: window hash, headless hash, `hash_match=yes`, `passes_byte_identical=yes`, the file's SHA-256, and the sim camera x at that tick).

The Ring rooms have no `--room-pilot`, and the walk-right script stops at the closed shutter, so a capture could not stand a hero at the full ring or the exit. `--ring-pilot` (RNG-AD18) plays the library pilot of `f01_ring_fixtures` in the scripted run (the step, the walkway, the fight, the exit). It reads the state only. The stills therefore show a run of the whole room, tick by tick, not a hand-posed frame.

| Still (tick) | Room | What it shows |
| --- | --- | --- |
| `entry_keystone` (40) | 4a | the entry: the bricked-up blind arch (it keeps the composer's `arch_ring`, so it reads as a lit blind arch, not a glowing hole), the timber door under its arch and the crowned apex stone on it, the soffit band and the upper arcs |
| `great_arc_shutter` (280) | 4a | the great arc, the step, the walkway ledge front, the stone shutter between its jambs under the housing |
| `walkway_under_arc` (340) | 4a | **gate C2e:** Carl on the walkway under the great arc (his head falls on the inner band at the crown) |
| `after_walkway` (530) | 4a | Carl settled at x 1365 with the camera at rest: the shutter, shield, fallen voussoirs and the arc's right leg (the diagram and shutter crop use this still) |
| `ring_goblin` (700) | 4a | **gate C2e:** the full ring entirely in frame with the floor, Carl and the goblin of x 1824 |
| `full_ring` (740) | 4a | the full ring on brick at x 1680..1888 (its bay suppressed and bricked up), the shelf and the torches of the pools (camera clamped at the room's end) |
| `exit` (850) | 4a | Carl at the `dfx_exit` door |
| `arc_route` (100) | 4b | the second arc over the overhead route, the entry |
| `camp` (280) | 4b | the camp outskirts: lean-to, bedroll, crates, lantern, the goblin of x 848 |
| `r3_breach` (340) | 4b | the R3 view: the breach with the far camp inside |
| `marker_exit` (380) | 4b | the shared marker and the exit, taken at tick 380 before Carl reaches the exit (from tick 390 the F01-RUN room-clear panel covers the view; it is not part of the kit) |

## Offline sheets

| File | Content |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | every piece at native size, and labelled at 4x (the parallax strips are in `parallax_x2.png`) |
| `shutter_states_x4.png` | the stone shutter through Open, Warning (ticks 0, 20, 35), Closing (17, 34, 51 px), Closed with the floor-contact puff, Closed at rest and Opening, drawn from the same piece crops the presenter uses (spec section 6). **Not runtime:** the court's shutter is static (RNG-AD6), so these states are unit-tested on a core `Shutter` and their in-scene capture follows RING-C wire |
| `layout_diagram_court_x2.png`, `layout_diagram_split_x2.png` | the schema at 2x over a real still (`after_walkway`, `marker_exit`: the camera is at rest, so the sim camera x equals the view's left): walking line W 580, floor back line FB 560, the soffit band 336..352 (the ceiling slab solid), every dressing rect, the platforms and the shutter rect |
| `crop_shutter_court_x4.png` | a 4x crop of the court's shutter, housing and jambs from the real `after_walkway` still |
| `parallax_x2.png` | the three parallax strips: `ring_tiers_far`, `ring_tiers_mid`, `ring_camp_far` |
| `measurements.txt` | the lit-render measurements below |
| `gate/` | the silhouette gate sheets regenerated after K1..K4 (`python3 scripts/author_rings01.py gate docs/evidence/area-rings-01/gate`), with `measurements.txt` (K1, K2) |

## Measurements

`gate/measurements.txt` (palette L* of the opaque pixels; the brick is the existing court still's lit brick). The keys were re-frozen by the design fix round (torch bake, running-bond shutter), so these are the re-measured numbers:

| Gate edit | Target | Measured |
| --- | --- | --- |
| K1 `ring_arc_wall_a` | arc opaque mean L* at most 31; stone 7 and 8 absent; stone 6 at most 3 % | mean L* 24.1 (crown rows 20.2, leg rows 27.2; the torch bake of RNG-AD22 lifts the legs in their pools); stone 7 and 8 absent; stone 6 on 1.72 % |
| K2 `ring_shutter_closed` | key mean L* at most 19, at least 8 below the brick beside it | key mean L* 17.9 (before K2: 24.8), slab stone only 19.2; brick beside the shutter 28.5, so the key is 10.6 below it and the slab stone 9.3 below |

`measurements.txt` (`python3 scripts/measure_rings01.py DIR`: the kit's own method, `scripts/env_kit_light.py`, with the real composed layouts, plain against dressed; mean CIE L* of the actor pixels minus a 6 px ring around the actor box):

| Gate constraint | Measured |
| --- | --- |
| C4 Carl on the walkway at x 1216 | dL* **20.5** dressed, 15.3 on the plain room (the kit's accepted value is about 8.6) |
| C4 goblin at x 1824 in front of the full ring (now at ring-local x about 144) | **9.5** dressed, 8.3 on plain brick in the same light. `local_dark` of `ARC_LM` is recomputed after the move to 1680: a per-band drop (3, 3, 2 steps) over piece-local x 112..180, y 160..204 with an 8 px dithered falloff, applied after the torch bake, so there is no hard-edged wedge. The first attempt (3, 2, 2 steps, applied before the bake) read 8.2, just under plain brick |
| the other goblins (4b x 560 and 848) | 8.1 and 9.6 dressed against 6.8 and 6.5 plain |
| goblin at 4a x 1424 | no worse than plain brick | **4.1** dressed, 2.1 plain (the final torch spots of RNG-AD24: the free torch at (1360, 368) and the one at (1656, 448)) |
| C1 walkable tops | the top arris is the brightest 13 px run within 64 px | true on all five platforms (step 44.5 against 40.6, walkway 40.6 against 38.9, shelf 50.8 against 41.9, route 46.7 against 39.7, R3 ledge 40.8 against 39.7) |

## Design fix round: what the new captures show

- **The bricked-up entry arch** keeps its `arch_ring_a`, so its two gallery lights still fall on it. In `scene_entry_keystone_*` it reads as a lit blind arch of brick with the chain swag, not as a glowing hole: the infill is the same brick as the wall, and no gallery colour shows through.
- **4b** keeps the composer door on the pier and has no keystone, as intended (`scene_arc_route_*`).
- **The great arc's legs** take the torch bake in the pools of the torches: the legs read as lit stone in their pools, no longer as flat dark cut-outs. **The torches (RNG-AD24):** the raised torch at (1360, 368) reads as a wall torch over the broken leg, clear of the soffit and of the arc's shoulder (`scene_walkway_under_arc_*`, `scene_after_walkway_*`); the torch at (1656, 448) stands clear of the ring's left edge (`scene_ring_goblin_*`).
- **The full ring** (`scene_ring_goblin_*`, `scene_full_ring_*`) sits entirely on brick at 1680..1888 with its whole circumference visible and a transparent interior: it reads as a carved ring.
- **The shutter** is a running-bond masonry slab with an iron band and shoe and no straps. Owner question (not blocking): if it still reads door-like, the next step is to lower the band to the shoe.

## Design notes returned to the designer (not decided here)

None open from this round. The bedroll's curl line is faint at 1x (accepted by the review, no action).

## NOT RUN

The moving shutter in a scene (the court's shutter is static; RING-C wire), a human-played run, a pad, a release build, the 640x400 height with a hero on the R3 ledge (the pilot walks the floor), and any owner review.
