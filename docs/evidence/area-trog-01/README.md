# AREA-TROG-01: the troglodyte run and the Test of Strength (rooms `trog_run` and `trog_strength`)

Evidence for [the AREA-TROG-01 spec](../../02-art/area-trog-01-spec.md). **Stage 1 (`trog_run`, merged as #311) and stage 2 (`trog_strength`) are executed**: the kit drawn and wired into both rooms, the real window captures, the offline sheets and the measurements, plus the silhouette gate sheets of the design step (`gate/`). In stage 2 the bench is the intact key placed statically on TROG-B's 80 px static solid (TROG-AD46, TROG-ART-AD-S2); the weakened, broken and debris keys are reserved for TROG-Q14 and appear in no still, atlas or contract. Everything here is **prototype art** (`prototype_fixture_unapproved`): the independent art review is pending and **nothing is owner-approved**. Game feel and the owner's art review come after the merge.

## Real Bevy captures (`scene_*`)

Each still is TROG-A's / TROG-B's capture command rendered in a real window under xvfb, in two sizes: `640x360` (`--scale 1`) and `1920x1080` (`--scale 3`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S \
  --scene f01_late --f01-room trog_run|trog_strength --trog-pilot carl --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`scripts/make_trogarea01_evidence.sh` captured every still **twice** (two full passes): the two PNGs are byte-identical, and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`): the art is presentation only. Where TROG-A ([`../trog/capture_lines.txt`](../trog/capture_lines.txt)) or TROG-B ([`../trog-b/capture_lines.txt`](../trog-b/capture_lines.txt)) captured the same pilot, tick and size, the hash also equals theirs (`troga_match=yes` on 18 lines, `trogb_match=yes` on 8, never `no`; `n/a` for the new ticks). [`capture_lines.txt`](capture_lines.txt) holds one `TROG_ART_STILL` line per still and size. The camera eases per frame, so the drawn view of each 640 x 360 still is **measured from landmarks** (the kit's lit model slid over the still's wall rows, `scripts/make_trogarea01_sheets.py`, cross-checked on the torch flames: flame centre = placement x + 7.5; the logged `cam` is not the drawn view) and is in [`drawn_views.txt`](drawn_views.txt).

| Still | Pilot tick | Drawn view (left, top) | What it shows |
| --- | --- | --- | --- |
| `t0_spawn` | 20 | 0, 292 | T0: the slam crack high on bricked bay 1, the knuckle marks low beside the door, the low refuge's stone face, the Pygmy asleep at the right edge |
| `pygmy_dormant` | 80 | 11, 292 | the same bay and the refuge with Carl and Donut, the Pygmy asleep |
| `pygmy_tell` | 125 | 101, 260 | the Pygmy's coil, the refuge face and the crater under it |
| `pygmy_leap` | 144 | 95, 240 | mid-leap over the crater |
| `pygmy_land` | 152 | 96, 260 | the Pygmy down by the refuge after its leap; the crater of earlier landings is at its spawn behind it |
| `basher_dormant` | 440 | 764, 292 | T2: the fist-shaped hole in bricked bay 4 behind the Basher's spawn, the Basher in front of it, never inside it |
| `basher_tell` | 520 | 949, 292 | the overhead tell in front of the hole |
| `basher_smash` | 543 | 949, 292 | the smash |
| `virtuoso_dormant` | 840 | 1623, 228 | T3: the bell on its bracket, the bell step face, the Virtuoso standing |
| `virtuoso_tell` | 870 | 1664, 292 | the feint, with the bell and the step |
| `virtuoso_lash` | 893 | 1664, 292 | the lash |
| `exit_open` | 960 | 1664, 292 | the troglodytes beaten (the downed Virtuoso is not counted in the lit read), the exit open |
| `strength_open` | 30 (`trog_strength`) | 0, 292 | the Test of Strength's opening view: the 48 px retreat shelf's stone face and the Virtuoso asleep at 496 (the view ends at x 639, so the bench, which starts at 640, is just off the right edge) |
| `strength_virtuoso_tell` | 105 | 78, 292 | the Virtuoso's feint in front of the shelf |
| `strength_bench_beside` | 230 | 231, 292 | Carl beside the intact bench; the threshold under its far side, the gym bay with the cold tube, the girders, the shards in their frame, the rack and the plate tree; the Basher asleep in front of the bay, the Virtuoso down at the left |
| `strength_bench_climb` | 270 | 318, 292 | Carl landing on the bench top (his contact shadow is on the top) |
| `strength_basher_tell` | 360 | 375, 292 | the Basher's overhead tell in front of the gym bay |
| `strength_basher_smash` | 381 | 375, 292 | the smash |
| `strength_exit_open` | 450 | 384, 292 | both defeated, the exit open |

## Offline sheets and measurements

| File | Content |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | every piece of both rooms at native size (labelled at 4x): the keys of the run and of the strength room (the bench is the intact key only) and the executor step and shelf pieces |
| `layout_diagram_t0_spawn_x2.png`, `layout_diagram_pygmy_land_x2.png`, `layout_diagram_basher_dormant_x2.png`, `layout_diagram_virtuoso_dormant_x2.png`, `layout_diagram_strength_open_x2.png`, `layout_diagram_strength_bench_beside_x2.png`, `layout_diagram_strength_bench_climb_x2.png` | the schema at 2x over the real stills: the walking line W 580, the floor back line 560, the open windows with the y 520 limit, the bricked bays' fill rects, every dressing rect, the troglodytes' spawn x (in the strength room the shelf face, the mats and the bench are labelled) |
| `lit_read.txt`, `lit_read_masks_x2.png` | TROG-ART-A12 on the real stills (gate note G6): the troglodytes' actor L* and edge contrast and every dressing piece's lit mean L*; the masks over the stills |
| `measurements.txt` | sizes, colours, mean luma of the executor pieces, the visibility of every placed piece (TROG-ART-A13) and the sixteen key pins |
| `drawn_views.txt` | the measured view left and top of every 640 x 360 still, with the torch-flame cross-check |
| `SHA256SUMS` | every file of this folder except `gate/` (which has its own) |

## Measurements

- **TROG-ART-A12, the lit read (gate note G6): passes on every gated still.** The gated stills are those with a troglodyte in an idle pose in view: `t0_spawn`, `pygmy_dormant`, `basher_dormant`, `virtuoso_dormant`. The troglodytes are located by masked normalised cross-correlation of their atlas frames against the still (pose and position are not logged; ncc 1.00 on every gated still).

  | Gated still | Troglodyte | actor L* | edge_dL | Dressing in view, lit mean L* |
  | --- | --- | --- | --- | --- |
  | `t0_spawn` | Pygmy | 29.0 | 18.1 | slam crack 17.5, knuckle marks 20.0, step faces 17.3..22.2, crater 19.1 |
  | `pygmy_dormant` | Pygmy | 28.2 | 19.3 | slam crack 17.3, knuckle marks 20.0, step faces 14.2..21.8, crater 19.1 |
  | `basher_dormant` | Basher | 33.6 | 25.7 | fist hole 12.5 |
  | `virtuoso_dormant` | Virtuoso | 25.4 | 18.6 | bell 24.1, step faces under 22.2 |

  The thinnest margin is the bell against the Virtuoso (24.1 against 25.4, 1.3 L*). Attack and walk poses are reported, not gated, and also pass (`lit_read.txt`). Carl is brighter than any troglodyte (TROG-ART-Q6), which is not gated.
- **TROG-ART-A12 in the Test of Strength (gate note G6): passes.** Gated stills: `strength_open` (Virtuoso asleep), `strength_bench_beside` and `strength_bench_climb` (Basher asleep). Every new or reused dressing piece in view (the cold tube excluded, it is emissive) is below the lowest living troglodyte's actor L*:

  | Gated still | Troglodyte | actor L* | edge_dL | Dressing in view, lit mean L* |
  | --- | --- | --- | --- | --- |
  | `strength_open` | Virtuoso | 25.6 | 20.8 | shelf face 17.0..18.6 |
  | `strength_bench_beside` | Basher | 32.6 | 18.9 | bench 19.9, shards 11.9, threshold 18.1, rack 20.7, girders 17.0..19.5, mats 21.4..23.6, shelf face 16.6..18.0 |
  | `strength_bench_climb` | Basher | 32.5 | 19.6 | bench 19.2, shards 11.9, threshold 17.5, rack 20.9, plate tree 13.1, girders 15.3..19.5, mats 19.1..22.3, shelf face 17.0..18.7 |

  The thinnest margin is the shelf's mid against the Virtuoso (18.6 against 25.6 in `strength_open`, 7.0 L*; it was 3.2 L* before the torch above the shelf was removed, TROG-ART-AD-S7). The bench is at 19.2..19.9, well under the Basher (32.5) and the Virtuoso (25.6), so the torches removed above it and above the shelf (G1, AD-S7) cost no read. The downed Virtuoso is not counted (it is `down`). Carl is not gated.
- **TROG-ART-A13:** all 23 placed `trog_*` pieces (15 in the run, 8 in the strength room) are 100 % visible behind the architecture, and no backwall decal pixel is over a null tile; with the props in front the shards are 96.7 % visible (the rack) and everything else 100 %.
- **The keys:** the sixteen key pins in `measurements.txt` equal `gate/measurements.txt`; `tests/test_art_trogarea01.py` holds them and the eight key grids byte-identical. The weakened, broken and debris keys stay pinned and are not packed (TROG-Q14).

## Gate sheets (`gate/`, the design step)

The gate composites are scratch previews made before any room was dressed in code; they are left exactly as the designer committed them and are not reproducible from the tree. The runtime stills above replace them as evidence.

| File | Content |
| --- | --- |
| `keys_1x.png`, `keys_4x.png` | every key frame at native size and at 4x, in colour and as a black silhouette: the fist hole, the bell, the knuckle marks, the slam crack, the crater, the threshold, the mirror shards, the three bench states and the six debris frames |
| `bench_states_debris_lit_3x.png` | the bench in its three states and the debris burst frame by frame, lit by the kit's light model, at 3x |
| `gate_overview.png` | the whole gate on one sheet: the five views lit (left) and unlit (right), the key sheet in colour and silhouette, and the bench states and debris burst lit |
| `run_t1_{lit,unlit}_1x.png` | `trog_run`, view left 96: the slam crack and knuckle marks on bricked bay 1, the low refuge face, the crater and the Pygmy |
| `run_t2_{lit,unlit}_1x.png` | `trog_run`, view left 1056: the fist hole on bricked bay 4 with the Basher in front of it |
| `run_t3_{lit,unlit}_1x.png` | `trog_run`, view left 1664: the bell, the bell step face and the Virtuoso |
| `strength_entry_{lit,unlit}_1x.png` | `trog_strength`, view left 0: the spawn, the retreat shelf face and the Virtuoso |
| `strength_bench_{lit,unlit}_1x.png` | `trog_strength`, view left 384: the bench stack (intact), the threshold, the gym bay (girders, cold tube, mirror shards, rack, plate tree, mats), the Virtuoso and the Basher |
| `hero_*_lit_4x.png` | 4x crops of the lit views: the marks, crater and Pygmy; the fist hole and Basher; the bell and step; the shelf, threshold and Virtuoso; the bench, gym bay and Basher |
| `measurements.txt` | the key measurements (size, colours, luma, bounds), the SHA-256 pins of the sixteen key frames, the lit read per view (troglodyte actor L*, edge contrast, each dressing piece's lit mean L*), and the visibility of each placed piece |

**How the composites were made.** The two rooms were composed by the real Rust composer (the plain TROG-A composition), dressed through a scratch contract with `ring_kit::dress_with` (the placements, removals and bricked bays of spec sections 4 and 5), and dumped as layouts by a scratch test. A scratch script rendered each layout with the kit renderer, lit (`env_kit_light.render_lit`) and unlit. It then pasted frame 0 of each troglodyte (facing left) and of Carl (facing right) at their spawn x with the feet on y 579, and the bench at its origin. The six step and shelf faces are scratch versions built by the rule in spec section 5. Views are 640 x 360 with the top at y 292. The probe and the script are not committed. These stills are a design preview, **not runtime evidence**; the execution replaces them with real window captures.

**Key pins.** `python3 scripts/author_trogarea01.py keys` prints the sixteen pins; they must equal the `KEY` lines of `gate/measurements.txt`. `python3 scripts/author_trogarea01.py measure` prints the key measurement lines.

Status: prototype art, independent art review pending, not owner-approved. [`gate/SHA256SUMS`](gate/SHA256SUMS) lists every gate file.
