# DEATH-C evidence: the cosmetic death presenter in the real Bevy scene

**Status:** real `floor01_scene` captures (xvfb, the real render loop, lighting on, one frame per tick) of the goblin crush and the kick kill at gore preset 4 (the default) and preset 1. **Nobody has judged the look or the feel yet; owner game-feel review and independent art review are pending.** Design and behaviour: [DEATH-C in the floor 01 scene doc](../../03-tech/floor01-scene.md#death-c-cosmetic-death-presenter-and-gore-pool). Art: [GOB-02-DEATH](../../02-art/gob-02-death-spec.md).

Regenerate with `cargo build --locked --bin floor01_scene` then `python3 scripts/make_death_c_evidence.py` (needs `xvfb-run`, `ffmpeg`, Pillow). The per-tick PNG frames stay in `artifacts/death-c` (git ignored); the MP4 files contain every one of them.

## Replay is identical at every preset

Every capture run prints its final `COMBAT_STATE`; the script fails unless it equals the headless replay of the same tick, at both presets and both resolutions: `cp03_shore_kill` to tick 239 (hash printed by the capture run), `cp01_kick` to tick 340 `hash=9e13d64619149b71` (the committed fixture hash). Unit tests and `ci_floor01_combat.sh headless` also run every fixture at presets 1 to 4 against the committed replays.

## Clips (full speed)

| File | Content |
| --- | --- |
| `crush_p4_640x360_60fps.mp4`, `crush_p1_640x360_60fps.mp4` | `cp03_shore_kill`, ticks 110 to 239 (the shore is broken on tick 103, the block falls and lands on tick 139), 640x360, 60 fps = real time |
| `crush_p4_1920x1080_60fps.mp4`, `crush_p1_1920x1080_60fps.mp4` | the same at 1920x1080 (`--scale 3`) |
| `kick_p4_*`, `kick_p1_*` (640x360 and 1920x1080) | `cp01_kick`, ticks 256 to 340 (the killing kick lands on tick 260: recoil, launch, flight, landing, pool) |
| `crush_p1_vs_p4_640x360_30fps.gif`, `kick_p1_vs_p4_640x360_30fps.gif` | preset 1 (left) next to preset 4 (right), every second tick at 33 ms (real time) |
| `sheet_crush_p4_vs_p1.png`, `sheet_kick_p4_vs_p1.png` | 1920x1080 crops at key ticks, preset 4 left, preset 1 right (the crush sheet: before, contact, flatten, remains, pool stage 2, stage 3, L+90) |
| `still_*.png` | full 1920x1080 captures of key ticks |

## What the captures show

* Crush: `crush_contact` on the landing tick with the club leaving the hand and blood squeezed out at the mouth and both squeeze points, the flatten frame, then `crushed_remains` held; at preset 4 the wall spatter, splats, medium splats and the pool growing to stage 3 (38 px); at preset 1 the same poses and silhouettes with the gore layer hidden, the club still clattering, and no blood sprite anywhere.
* Kick kill: the GOB-01 knockback and corpse frames stay; at preset 4 a spray of drops from the lethal hit, splats where the body lands, a smear and pool stages; at preset 1 nothing is added.
* The debris and decals are drawn under Carl and Donut (unit-tested draw order); Carl is on screen in every frame.
* The approved ENV-02 shore, block, landed rubble and impact dust are drawn from the simulated shore state: the block falls (the shore breaks on tick 103, lands on 139) and the goblin is pressed under it; the remains lie in front of the rubble (art spec ruling 2). Preset 4 was raised after the independent review: crush pool stages on L+2, L+10, L+24 up to the 38 px pair, three wall spatters (both variants), the kick kill with 24 drops, gibs, a bone, a wall spray, a smear and three pool stages.
* The wall spatter is still the spec's small art on lit brick (the art spec's open note); three overlapping copies read as a splash, judge it in the clips.
* The decapitation was captured while developing with the lab flag `--death-lab-finish decapitated` (not committed as evidence; there is no authoritative cause before PR D).

## Frame times with the pools at max load

`perf_frame_times.txt` (raw lines). Debug binary (opt-level 1), **software rendering under xvfb (llvmpipe)**: the numbers are CPU raster time, not a GPU or Steam Deck figure, and the machine was shared and noisy (the preset 1 stress run is slower than preset 4 only because of that noise). `--death-stress` keeps parts 160/160, floor decals 96/96 and wall decals 12/12 loaded (pieces recycled thousands of times). Medians over 600 frames after 120 warm-up frames:

| Resolution | baseline (no death) | stress preset 4 | stress preset 1 |
| --- | --- | --- | --- |
| 1920x1080 | 76.7 ms (p95 91.6) | 78.0 ms (p95 91.6) | 83.2 ms (p95 115.9, noisy) |
| 640x360 | 16.6 ms (p95 23.5) | 17.1 ms (p95 25.7) | 18.3 ms (p95 34.5, noisy) |

A separate quiet run during development measured 56.8 ms baseline against 57.2 ms at stress, preset 4, at 1920x1080. The presenter's own CPU cost with every pool full is about 6 microseconds per tick at preset 4 and 2 microseconds at preset 1 (ignored test `presenter_step_cost_at_max_load`). The budgets (160 / 96 / 12) were first guesses; [DEATH-E1](../death-e/README.md) measured them and raised them (256 / 192 / 32). No Steam Deck or lower-spec Linux measurement exists yet, and none is claimed.

## Rights/Provenance

Presentation code and evidence by Claude (medium tier, execution of the GOB-02-DEATH art spec); the art is GOB-02-DEATH (design Claude Opus, execution Claude Sonnet, see its spec) under the owner-reported `ART-PUBLICATION-ALL-V1` scope, unchanged here. No new art was drawn; every pixel in the captures is the committed GOB-02 and GOB-01, Carl, Donut and Floor 1 art. Independent top-tier review of this integration: pending.
