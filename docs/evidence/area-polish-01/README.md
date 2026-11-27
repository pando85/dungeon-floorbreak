# AREA-POLISH-01 evidence

Evidence for [the AREA-POLISH-01 spec](../../02-art/area-polish-01-spec.md): the mastery arena (room 17) and the Dozer's room shell (room 8) dressed from approved pieces, the corral's (room 4) M3 grate art and partition splinter burst, and the Juicer's (room 15) exit lintel. **Owner art review pending**; nothing here is accepted, and game feel is the owner's call.

Every still is the real Bevy window of `scripts/room_capture.sh ROOM PLAN FIRST:STEP:COUNT` (`--scene f01_late --f01-room ROOM --room-pilot PLAN`, the library pilots) under xvfb at `--scale 1` (640 x 360); the still is the **last frame** of its sequence (the camera eases per frame). `scripts/make_polish01_evidence.sh` captured each one **twice** and the two PNGs are byte-identical (`passes_byte_identical=yes`, 13 of 13), and the window's final `COMBAT_STATE` hash equals the headless run's at the same tick (`hash_match=yes`, 13 of 13). The dressing is presentation only: the Juicer's tick-60 and tick-3230 hashes (`9020c4f828cc5534`, `bb6db258476bc97e`) are the ones of the [gym evidence](../area-gym-01/README.md), and before the dressing a `floor01_scene` built from origin/main gave the same final hash for the Dozer's `dozer:run` run at tick 840 (`371e3aeb7f854b73`), the corral's `corral` run at tick 960 (`963f95f07c8fbf51`) and a mastery run to tick 2160 (`108deff92269a0a6`). [`capture_lines.txt`](capture_lines.txt) holds one `POLISH_STILL` line per still.

Regenerate with `bash scripts/make_polish01_evidence.sh` (needs the debug binary, `scripts/export_assets.sh`, xvfb, flock and ImageMagick). Captions claim only what is in frame.

## Stills (`scene_*_640x360.png`)

| File | Room, plan, sequence (tick) | What is in frame |
| --- | --- | --- |
| `scene_mastery_spawn_*` | mastery, `mastery`, 60:30:3 (120) | The composed first bay at the left (its open gallery, the wooden door, a torch), Carl, Donut, the slime and goblins at the spawn, and the first blind bay at the right: the arch ring and pier over brick, with one wall cavity inside it |
| `scene_mastery_arcade_*` | mastery, `mastery`, 330:30:3 (390) | Three arches in a row (the composed bay's right edge and the two blind bays), torches on the piers, the two wall cavities, goblins and a llama on the lane |
| `scene_mastery_exit_*` | mastery, `mastery`, 900:30:3 (960) | The second blind bay with its cavity and the exit door at the right edge, the heroes at the door |
| `scene_dozer_bricked_bay_*` | dozer, `dozer:run`, 60:30:3 (120) | Bay 1's open arch, the bricked bay 2 with its wall cavity and no sill under it, bay 3's open arch at the right with the stop frame's weight rig hanging at the top right |
| `scene_dozer_chase_*` | dozer, `dozer:run`, 480:30:3 (540) | The DOZ-01 machine at the left edge (untouched), Donut and Carl on the lane, the tunnel's lintel tiles and pipe and the exit door at the right |
| `scene_dozer_tunnel_*` | dozer, `dozer:run`, 780:30:3 (840) | The machine stopped at the left, the tunnel and the exit door at the right |
| `scene_corral_grate_down_*` | corral, `corral`, 930:30:2 (960) | The M3 gate drawn as the approved room 5 grate (vertical bars with bands), from the ceiling line down to the dock's top, with Carl, Donut and a llama in front of it and the exit gate at the right edge |
| `scene_corral_grate_open_*` | corral, `corral`, 1000:20:2 (1020) | The same wall after the llamas are defeated: the gate is open and nothing is drawn where the grate stood |
| `scene_corral_partition_standing_*` | corral, `corral_partition`, 600:20:2 (620) | The pen with the partition standing (the timber screen on the lane) before the break, Carl and Donut, a llama |
| `scene_corral_partition_burst_a_*` | corral, `corral_partition`, 660:4:6 (680) | 7 ticks after the break (tick 673): small splinter flecks beside the remnant, in front of Carl's fist and the llama |
| `scene_corral_partition_burst_b_*` | corral, `corral_partition`, 660:4:7 (684) | 11 ticks after the break: the second burst's flecks, the first burst on its third frame |
| `scene_juicer_exit_closed_*` | juicer, `juicer`, 60:1:1 (60) | The right end of the arena: the closed exit door under the second steel lintel (a riveted header over the end-wall column, its bottom row on the door's top), the right mirror bay below it and a torch on the brick above |
| `scene_juicer_exit_open_*` | juicer, `juicer`, 3230:1:1 (3230) | After the defeat: the exit open on its warm passage under the same lintel |

## Not run

- A lit-read measurement of the new pieces on these stills (the dressing was judged by eye at 1x and by the geometry tests of the spec, section 5).
- Human recognition and the owner's art review: **NOT RUN** (pending).
- Real-display or handheld frame time: **NOT RUN**.
- The partition burst is small at 1x (48 px frames, a few specks): whether it reads as a break is the owner's call (spec POLISH-AD5).
