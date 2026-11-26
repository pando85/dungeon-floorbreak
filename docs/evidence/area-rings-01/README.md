# AREA-RINGS-01: the Broken Ring kit (rooms 4a and 4b)

Evidence for [the AREA-RINGS-01 spec](../../02-art/area-rings-01-spec.md): the kit drawn and wired into both rooms, the stone shutter presenter, the real captures and the measurements. Prototype art (`prototype_fixture_unapproved`): the silhouette gate passed with the key edits K1..K4 (spec section 16), the independent review is pending, and nothing here is owner-approved. Game feel is the owner's call.

## Real Bevy captures (`scene_*`)

Every still is the real window of `--scene f01_late --f01-run --f01-room ROOM --ring-pilot`, under xvfb, in three sizes: `640x360` (`--scale 1`), `1920x1080` (`--scale 3`) and the `640x400` deck view (`--deck --scale 1`). `scripts/make_rings01_evidence.sh` captured each one **twice** (two full passes) and the two PNGs are byte-identical; the window's final `COMBAT_STATE` hash equals the headless run's at the same tick for every line. [`capture_lines.txt`](capture_lines.txt) holds one `RINGS_STILL` line per still and size (30 lines: window hash, headless hash, `hash_match=yes`, `passes_byte_identical=yes`, the file's SHA-256, and the sim camera x at that tick).

The Ring rooms have no `--room-pilot`, and the walk-right script stops at the closed shutter, so a capture could not stand a hero at the full ring or the exit. `--ring-pilot` (RNG-AD18) plays the library pilot of `f01_ring_fixtures` in the scripted run (the step, the walkway, the fight, the exit). It reads the state only. The stills therefore show a run of the whole room, tick by tick, not a hand-posed frame.

| Still (tick) | Room | What it shows |
| --- | --- | --- |
| `entry_keystone` (40) | 4a | the entry bay: the crowned keystone over the timber door, the parallax tiers through the arch, the soffit band and the upper arcs |
| `great_arc_shutter` (280) | 4a | the great arc, the step, the walkway ledge front, the stone shutter between its jambs under the housing |
| `walkway_under_arc` (340) | 4a | **gate C2e:** Carl on the walkway under the great arc (his head falls on the inner band at the crown) |
| `after_walkway` (530) | 4a | Carl settled at x 1365 with the camera at rest: the shutter, shield, fallen voussoirs and the arc's right leg (the diagram and shutter crop use this still) |
| `ring_goblin` (700) | 4a | **gate C2e:** the full ring entirely in frame with the floor, Carl and the goblin of x 1824 |
| `full_ring` (740) | 4a | the full ring, the shelf, the exit pier (camera clamped at the room's end) |
| `exit` (850) | 4a | Carl at the `dfx_exit` door |
| `arc_route` (100) | 4b | the second arc over the overhead route, the entry |
| `camp` (280) | 4b | the camp outskirts: lean-to, bedroll, crates, lantern, the goblin of x 848 |
| `r3_breach` (340) | 4b | the R3 view: the breach with the far camp inside |
| `marker_exit` (420) | 4b | the shared marker and the exit (the room-clear panel is the F01-RUN panel, not part of the kit) |

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

`gate/measurements.txt` (palette L* of the opaque pixels; the brick is the existing court still's lit brick):

| Gate edit | Target | Measured |
| --- | --- | --- |
| K1 `ring_arc_wall_a` | arc opaque mean L* at most 31; stone 7 and 8 absent; stone 6 at most 3 % | mean L* 23.2 (crown rows 19.4, leg rows 26.3; before K1: 35 and 39 as the gate measured, 31.4 whole-piece); stone 7 and 8 absent; stone 6 on 1.72 % |
| K2 `ring_shutter_closed` | key mean L* at most 19, at least 8 below the brick beside it | key mean L* 17.7 (before K2: 24.8), slab stone only 19.0; brick beside the shutter 28.5, so the key is 10.9 below it and the slab stone 9.5 below |

`measurements.txt` (`python3 scripts/measure_rings01.py DIR`: the kit's own method, `scripts/env_kit_light.py`, the A1 lighting model on the exported atlases with the real composed layouts, plain against dressed; mean CIE L* of the actor pixels minus a 6 px ring around the actor box, `lookdev_compose.readability`):

| Gate constraint | Measured |
| --- | --- |
| C4 Carl on the walkway at x 1216 (head on the inner band and the crown's cast shadow) | dL* **20.1** dressed, 15.3 on the plain room (the kit's accepted value is about 8.6) |
| C4 goblin at x 1824 in front of the full ring | **8.9** dressed, 8.3 on plain brick in the same light. The first draw read 6.5 (worse than plain brick), so the landmark's band behind that head line is darkened three steps locally (`local_dark` in `scripts/author_rings01.py`, as C4 allows) |
| the other goblins (4a x 1424, 4b x 560 and 848) | 3.6 / 8.1 / 9.6 dressed against 2.1 / 6.8 / 6.5 plain: no worse anywhere |
| C1 walkable tops | in the lit render of every platform (the step, the walkway, the shelf, the overhead route, the R3 ledge) the top arris is the brightest 13 px horizontal run within 64 px vertically; on the plain rooms the brightest run is the deck (L* 41 to 43 at y 545) in every case. The torch-pool brick of the real window is not in this render |

## Design notes sent back to the designer (not decided here)

1. **The crowned keystone floats.** At (216, 440) it sits in front of the ground bay's opening, where the composer clears the backwall, so it hangs in the air over the gallery view, half behind the pier (see `scene_entry_keystone_*` and `scene_arc_route_*`). It needs a wall to sit on (a lintel, or a position over a solid backwall).
2. **The full ring overlaps the bay's pier and arch ring.** `ring_relief_landmark` (1656, 356) fills the bay opening exactly (208 px), but the composer's `pier` and `arch_ring` pieces (structure layer) draw over its left side and top, so the whole outer circumference is not visible even though the piece's own rows satisfy gate C2a (see `scene_ring_goblin_*`, `scene_full_ring_*`). Its interior shows the parallax gallery, not brick: AD2 and gate C2b disagree for this placement.
3. **The bedroll's curl is faint at 1x.** The rolled end is a lighter 4 px cap with a 6 px dark curl line; at gameplay scale it reads as a green roll with two ties, not a creature (K4's check), but the curl itself needs 3x to be seen.

## NOT RUN

The moving shutter in a scene (the court's shutter is static; RING-C wire), a human-played run, a pad, a release build, the 640x400 height with a hero on the R3 ledge (the pilot walks the floor), and any owner review.
