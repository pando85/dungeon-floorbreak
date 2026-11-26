# AREA-CAMP-01: the survivors' camp and the quiet aftermath (rooms `rest_camp` and `rest_aftermath`)

Evidence for [the AREA-CAMP-01 spec](../../02-art/area-camp-01-spec.md): the kit drawn and wired into the two Floor 01 quiet rooms, the real captures, the offline sheets and the measurements, plus the silhouette gate sheets of the design step (`gate/`). Everything here is **prototype art** (`prototype_fixture_unapproved`): the independent art review is pending, and **nothing is owner-approved**. Game feel and the owner's art review come after the merge.

## Real Bevy captures (`scene_*`)

Each still is the REST-WIRE capture command rendered in a real window under xvfb, in two sizes: `640x360` (`--scale 1`) and `1920x1080` (`--scale 3`):

```text
xvfb-run -a -s '-screen 0 1920x1080x24' floor01_scene --no-audio --no-record --scale S \
  --scene f01_late --f01-room rest_camp|rest_aftermath --rest-pilot walk|punch --companion off \
  --capture-sequence TICK:1:1 --capture-dir DIR
```

`scripts/make_camp01_evidence.sh` captured every still **twice** (two full passes): the two PNGs are byte-identical, and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`): the art is presentation only. [`capture_lines.txt`](capture_lines.txt) holds one `CAMP_STILL` line per still and size, with Carl's x. No REST-WIRE capture shares a room, pilot and tick with these stills (REST-WIRE's camp walk stills are at ticks 70, 200 and 390), so `rest_wire_match` is `n/a` on every line. The drawn view of each 640 x 360 still is **measured from landmarks** (the kit's lit model slid over the still's wall rows, `scripts/make_camp01_sheets.py`; the logged `cam` is not the drawn view) and is in [`drawn_views.txt`](drawn_views.txt).

**Correction of the design stage.** The gate's quoted view lefts (237, 483, 607 at ticks 161, 241, 281) were 25 px short: the matched drawn view is 262, 508 and 632 (tick 41 is 0 in both). The sentry's spear on the camp still at tick 161 confirms it (the spear, placement x 502, is at screen column 240). Only the gate composites used the old numbers.

| Still | Room, pilot, tick | Drawn view left | What it shows |
| --- | --- | --- | --- |
| `camp_entrance` | camp, `walk`, 41 | 0 | the marker, the lean-to and the sentry facing the entrance |
| `camp_sentry` | camp, `walk`, 161 | 262 | the sentry at the pilaster, the crates and the hearth in the distance |
| `camp_figures` | camp, `walk`, 241 | 508 | the crates, the hearth and its fire, the repair figure in window B3, the seated figure and the toy cart at the right |
| `camp_hearth` | camp, `walk`, 281 | 632 | the hearth with the fire lit (gate note G8), the repair, seated and map figures, the trestle |
| `camp_exit` | camp, `walk`, 361 | 879 | the map table and figure, the strength motifs, the chalk tally and the floor arrows toward the portal |
| `aft_entry_swing` | aftermath, `walk`, 21 | 0 | the egress (tyres, bench, the wreck); the bag's cap at the right edge during the entry swing |
| `aft_egress` | aftermath, `walk`, 41 | 0 | the same view after the swing |
| `aft_wreck` | aftermath, `walk`, 161 | 262 | the torn mirror wreck, the bricked bay with the girder run, the bag at rest, the sand under it |
| `aft_lane` | aftermath, `walk`, 281 | 632 | the plate tree and the chalk ticks and chevron of the blink lane |
| `aft_grooves` | aftermath, `walk`, 349 | 640 | the grooves round the exit portal |
| `aft_punch_*` | aftermath, `punch`, 163 to 194 | 260 | Carl standing at the bag and jabbing it: `rest` at 163, then the hit sway (`r1` 166, `r2` 170, `r1` 176, `l1` 184, `r1` 194). Cropped in `bag_sway_window_x4.png` |

`--rest-pilot punch` is new evidence tooling (aftermath only): Carl walks to x 608 and presses punch every 40 ticks; the jab is active on ticks 164 and 165. It touches no sim state and no existing pin.

## Offline sheets

| File | Content |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | every kit piece at native size (labelled at 4x): the five keys and the twelve executor pieces |
| `fire_loop_x4.png`, `bag_frames_x3.png` | the four `camp_fire` frames and the five bag frames |
| `bag_sway_window_x4.png` | the bag cropped from the six `scene_aft_punch_*` window stills: the presenter's frames in the real scene |
| `layout_diagram_camp_figures_x2.png`, `layout_diagram_camp_hearth_x2.png`, `layout_diagram_aft_wreck_x2.png`, `layout_diagram_aft_lane_x2.png` | the schema at 2x over the real stills: the walking line W 580, the floor back line 560, the window openings with the y 520 limit, every dressing rect, the fire's light origin (768, 548), the bag canvas and the sim's hit rect |
| `figures_lit.txt` | gate note G2: the four figures measured on the lit window stills |
| `measurements.txt` | sizes, colours, mean luma, the sway table and the key pins |
| `drawn_views.txt` | the measured view left and top of every 640 x 360 still |

## Measurements

- **Gate note G2 (lit legibility), all four pass.** On the lit 640 x 360 stills, with the figure in full view and the heroes clear of it:

  | Figure | (a) lit mean luma | (c) hood and shoulder contrast against the 2 px ring | (d) Carl, same still |
  | --- | --- | --- | --- |
  | sentry | 27.3 | 38.6 | 103.9 |
  | repair | 24.0 | 25.1 | 89.7 |
  | seated | 30.8 | 29.5 | 102.1 |
  | map | 26.2 | 70.7 | 104.3 |

  Targets: (a) at most 45 and below (d), (c) at least 20. No figure edge was changed and the sentry key is untouched. (d) is an estimate: the pixels the lit model of the room does not explain inside Carl's box. The repair figure's backdrop is the dark window B3, so it is the weakest read (25.1): an owner art note.
- **Gate note G5.** The repair figure's lamp holds one 2 x 2 `flame` 3 slit: no emitter, no animation, no `flame` 4 or 5 pixel (`tests/test_art_camp01.py`).
- **Gate notes G1, G3, G4.** The girder run meets both pier shadow columns (`camp_kit` test `the_girder_run_meets_both_piers`), the map figure's hand lies on the table's end (x 1152..1153), the cart is 7 empty columns from the seated figure.
- **Bricked bay (CAMP-A5, gate note G7).** No background pixel shows over x 488..743, y 352..559; the undressed room shows parallax in that span (`camp_kit` test).
- **Lights (CAMP-A7).** The camp trades four torches (a flame and a pool each) for the fire's two lights: minus 8, plus 2. The aftermath changes none. Both stay inside `MAX_LIGHTS` (`camp_kit` test).
- **Keys.** The twelve key frames are byte-identical to the pins in `gate/measurements.txt` (`tests/test_art_camp01.py`).

## Gate sheets (`gate/`)

The design stage's silhouette gate material, committed by the designer and kept as it was (the stills there are a design preview from a scratch probe build, not runtime evidence). `python3 scripts/author_camp01.py gate CAPS gate/` can regenerate them only from probe stills, which are not reproducible from this tree, so they are not regenerated.

## Regenerating

```text
cargo build --locked --bin floor01_scene
CAMP_LAYOUT_DIR=DIR cargo test --locked --lib -- --ignored dump_camp_layouts
bash scripts/export_assets.sh
CAMP_LAYOUT_DIR=DIR bash scripts/make_camp01_evidence.sh
```

Status: prototype art, independent art review pending, not owner-approved. [`SHA256SUMS`](SHA256SUMS) lists every file here.
