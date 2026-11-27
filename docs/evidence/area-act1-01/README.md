# AREA-ACT1-01 evidence

Evidence for [the AREA-ACT1-01 spec](../../02-art/area-act1-01-spec.md): the dressed act-1 rooms `act1_chase`, `act1_shore` and `act1_guild`, captured in the real Bevy window (xvfb, `--scale 1`, 640 x 360). **Owner art review pending**; nothing here is accepted, and game feel is the owner's call.

Every still is the act-1 capture command of [the ACT1 evidence](../act1/README.md) (`--scene f01_late --f01-room ROOM --act1-pilot PILOT --companion off --capture-sequence TICK:1:1`), captured **twice** (the two PNGs are byte-identical, `passes_byte_identical=yes`), and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (`hash_match=yes`, 14 of 14). Where the ACT1-A or ACT1-B evidence captured the same room, pilot and tick, the hash also equals theirs (`act1_match=yes`, 10 of 14; the other four are new ticks, `n/a`): the dressing is presentation only. Regenerate with `bash scripts/make_act1area01_evidence.sh` (needs the debug binary, `scripts/export_assets.sh`, xvfb, flock, ImageMagick, Pillow, NumPy, SciPy; `ACT1_LAYOUT_DIR` from the ignored test `act1_kit::tests::dump_act1_layouts` for the measurements).

## Stills (`scene_*_640x360.png`)

The drawn view (left, top) of each is in `drawn_views.txt`, measured against the lit render of the room (the camera eases, so the logged `cam` is not the drawn view). Captions claim only what is in frame.

| File | Room, pilot, tick | What is in frame |
| --- | --- | --- |
| `scene_chase_spawn_*` | chase, `hide`, 20 | Carl and Donut at the spawn, the catacomb arcade with torches. The breach, the soffit and the timber sets are not in frame (view x 0..639) |
| `scene_chase_s2_breach_heroes_*` | chase, `hide`, 235 | The bricked S2 bay (x 584..735) and the gallery breach from x 696 (view x 450..1089): its near-black void and its rubble foot, Donut and Carl in front of it; the machine has not emerged |
| `scene_chase_s2_emergence_*` | chase, `watch`, 470 | The machine (DOZ-01 rig) in front of the breach's void at the left edge, Donut and Carl right of it, a timber set intact (x 1104), the soffit's left end across the top |
| `scene_chase_s3_timbers_*` | chase, `hide`, 480 | The camera raised to Carl's jump: the soffit's flat underside across the top left, ending at x 1680; the second timber set (x 1424) intact with Carl and Donut on and by it; the torch removals leave the set clear |
| `scene_chase_s3_after_first_smash_*` | chase, `hide`, 510 | The first set (x 1104..1183) is already smashed and out of frame to the left (the camera is at x >= 1216 under every evidence pilot); the second set intact ahead, Carl and Donut |
| `scene_chase_s4_alcove_*` | chase, `hide`, 600 | The alcove niche on its stepped ledge (x 2064..2159) at the right, Donut and Carl on the lane |
| `scene_chase_s5_barricade_intact_*` | chase, `hide`, 825 | The machine, wheel and Donut in the alcove's arch; the barricade intact at the right (x 2304) |
| `scene_chase_s5_barricade_broken_*` | chase, `hide`, 880 | After the front's impact: the barricade's smashed frame (leaning planks) at the right, splinters at the front |
| `scene_chase_s5_after_defeat_*` | chase, `skip`, 925 | Carl defeated, the machine has broken the barricade during the defeat delay: the smashed frame stays shown |
| `scene_chase_s5_after_respawn_*` | chase, `skip`, 1350 | `TRY 2`: the respawn closed the lock and the barricade stands intact again; the heroes at the alcove |
| `scene_chase_wedge_*` | chase, `hide`, 1330 | The greybox wedge draw (the rig clipped and darkened at the arch's right edge), the exit portal at the right |
| `scene_shore_spawn_*` | shore, `strike`, 20 | The wedged machine at the left edge (the chase wedge's darkened rig, clipped to {56, 440, 192, 140}), Carl and Donut, the two goblins at the right |
| `scene_guild_door_closed_*` | guild, `donut`, 60 | The hearth and its flue, the barrel and stool at the left, the desk, the closed grant box, Mordecai, the bookshelf and wall map, the closed guild portal |
| `scene_guild_door_open_*` | guild, `donut`, 112 | The same room after the unlock: the grant box open, the portal's open overlay at the right |

## Sheets and tables

| File | Content |
| --- | --- |
| `contact_1x.png` | the 14 stills at 1x, two columns, labelled |
| `contact_x4_pieces.png` | the executed and derived pieces at x4: the gallery breach, the two grant box frames, the three step faces (mid, left end, right end), the alcove niche copy and the first soffit slice (a copy of the ENV-02 tiles) |
| `strip_breach_x2.png`, `strip_timbers_x2.png`, `strip_barricade_x2.png`, `strip_guild_x3.png` | crops of the stills above: the breach with the heroes and with the machine; a timber set intact and the first-smash still; the barricade intact, broken, after the defeat and after the respawn; the guild with the box closed and open |
| `measurements.txt` | the three `KEY` pins (equal `gate/measurements.txt`), the visibility of every placed piece and the lit read of spec section 10 on the real stills (masks: the pixels the lit model plus the presenter pieces explain) |
| `drawn_views.txt`, `capture_lines.txt` | the measured drawn views; one `ACT1_ART_STILL` line per still |
| `SHA256SUMS`, `gate/` | checksums; the design step's key sheets and pins |

## Readings (`measurements.txt`)

- **Visibility (target 5).** 39 placed pieces (23 in the chase, 16 in the guild; the shore places none) show 100.0 % of their opaque pixels behind the architecture with the props in front, and `backwall_null_px=0`; the bricked S2 bay x 584..735, y 352..559 has no background tile (`act1_kit` test A5 and this table).
- **S2 (target 2), passes.** Carl and Donut in front of the breach keep `edge_dL` 18.0 and 26.6; the machine's rig against the breach 18.4 (all >= 15); the void's lit mean L* is 2.2 over 6405 px (<= 12).
- **S3 (target 3), passes.** The timber set in view keeps `edge_dL` 21.6 against the soffit and the piers after the torch removal (>= 10; the first set is out of frame in the stills, so the second is the one measured). The spec's fallback torch is not used (ACT1-ART G5).
- **S4 (target 1), FAILS.** `act1_alcove_niche` has the lit mean L* 24.0 over its visible pixels; the brightest other shape is `pier_l` at 42.8 (margin -18.8). A model sweep of the glow (intensity 0.4 to 2.0, radius 96 to 200) lifts the niche to 47.7 at best while the piers it also lights climb with it, so the target cannot be met by tuning the light: see ACT1-ART-Q7 in the spec. For information only: the niche's 90th-percentile L* is 39.7 against `pier_r` 84.0.
- **Guild (target 4), FAILS two readings.** The grant box's lit mean L* is 16.5 (door closed) and 12.3 (open), below Donut's 42.3 and 41.9 (as required) but also below the desk's 21.8 and 21.3 (the target asks for >= the desk): the frozen key is a dark battered box. The hearth's lit mean L* is 43.7 and 42.7 against Mordecai's face region 43.4 and 33.6 (the top 12 rows of his component; an estimate, as the face is a few pixels): the hearth is not darker than the face. See ACT1-ART-Q7.

## Not run

- Human recognition of the breach, the alcove and the box, and the owner's art review: **NOT RUN** (pending).
- A window still of a timber set in its smashed frame: **not captured** (the front smashes the first set 112 px or more left of the camera under every evidence pilot, and the `watch` pilot is defeated before it reaches it). The frame is selected by `act1_kit::timber_piece` (unit-tested) and follows `Dozer::timber_smashed`, the flag the old flat rectangle followed.
- Real-display or handheld frame time: **NOT RUN**.
