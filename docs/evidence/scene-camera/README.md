# DEMO-01 PR C evidence: the automatic camera in the real scene

Real Bevy 0.20.1 captures and headless traces of `floor01_scene --camera auto`, the camera-lab operator ([camera lab](../../03-tech/camera-lab.md)) running as a presentation-only layer in the real scene ([Floor 01 scene > Camera](../../03-tech/floor01-scene.md#camera-demo-01-pr-c)). **Presentation only, no game-feel verdict.** Everything shows existing approved art (Carl, Donut, goblin A, the Floor 01 environment kit pieces).

**What the captures are.** The Bad Llama (PR B) is not on `main`, so nothing in the game threatens Carl from afar yet. The zoom is shown with a **synthetic foe that has taken aim** (`--camera-test-threat X:30:150`: a camera-only `Aiming` threat from elapsed tick 30 to 150, the state a llama takes before its tell). It has no sprite and no simulation: you see the camera react to a threat the player cannot see yet, which is the lab's priority 2. Two rooms: the **real `--demo` hall** (DEMO-01 PR A, 3200x640, `--demo --demo-beat 4`, Carl standing at the beat 4 checkpoint, foe at column 2560) and, for the rest of the matrix, the existing `cp01_haymaker_chain` fixture on a **tall camera-lab test room** (`--camera-test-room 1`: ROOM-01 of the lab composed from the approved kit, 1584x768, flat floor; the 2048x400 sample corridor is too short for any zoom). A capture with the real llama waits for PR B.

## Files

| Path | What |
|---|---|
| `frames/auto_zoomed_in_640_1920x1080.png` | Tick 12, before the foe takes aim: rest width 640, 3x scale, Carl, Donut and the goblin, lit. |
| `frames/auto_zoomed_out_960_1920x1080.png` | Tick 120, the foe is aiming: rest width 960, exactly 2x scale, the whole two-tier arcade above the action, lit with the A1 pass (torch pools, haze, contact shadows, parallax gallery). The HUD (PR #103) stays at its own 3x scale on its own camera while the world is at 2x. |
| `frames/auto_zoomed_out_960_nolighting_1920x1080.png` | The same moment with `--no-lighting` (the plain camera path): a `--capture-sequence 0:6:21` run, frame 20, tick 120, exactly like the lit frame. |
| `frames/demo_hall_auto_zoomed_in_640_1920x1080.png`, `frames/demo_hall_auto_zoomed_out_960_1920x1080.png` | **The real `--demo` hall** with the automatic camera (the demo default), tick 12 (640, 3x) and tick 120 (960, 2x), lit, HUD on its own camera. |
| `frames/scroll_camera_same_tick_1920x1080.png` | The existing scroll camera (`--camera scroll`) at the same tick: the default of every fixture, unchanged. |
| `frames/deck_auto_stays_640_1280x800.png` | Steam Deck output, the same run: 1280x800 is an exact 2x of 640 only, so the camera stays at 640 (no zoom, by the integer rule). |
| `frames/transit_strip_ticks_30_to_96.jpg` | Twelve captures, one every 6 ticks from tick 30: the eased, coupled zoom and pan (the width grows while the centre moves, the floor anchor sinks) through non-integer widths to the 960 rest. |
| `videos/demo_hall_auto_zoom_b4_10fps.mp4` | The same widen and return on the demo hall (60 real frames, one every 6 ticks, 10 fps playback). |
| `videos/auto_zoom_out_and_back_cp01_haymaker_chain_10fps.mp4` | 60 real frames, one every 6 ticks (10 fps playback, 960 px wide), tick 0 to 354: widen at 30, hold, pan back and return to 640 about 100 ticks after the foe stops aiming (tick 150). |
| `data/view_trace_auto_1080p.txt`, `..._4k.txt`, `..._deck_1280x800.txt`, `view_trace_scroll_1080p.txt` | `--view-trace`: one `FLOOR1_VIEW tick left top w h rest kick kick_px` line per tick of the same 420 tick run at 1920x1080, 3840x2160 and 1280x800 (automatic) and the scroll camera. |
| `data/view_trace_demo_autopilot_route_auto_1080p.txt`, `data/view_trace_demo_b4_auto_1080p.txt` | `--demo --view-trace`: the autopilot's recorded run of the route (2000 ticks) and of beat 4 with the synthetic foe at 1920x1080. No foe is in range of the autopilot's route, so both stay at 640 (no llama yet); the hall widening is in `hall_capture_camera_rest.txt` (the `--capture-sequence` run). |
| `data/hall_capture_camera_rest.txt` | `FLOOR1_CAMERA_REST` lines of the hall capture: rest 640 at tick 0, 960 at tick 30, back to 640 at tick 245. |
| `data/perf_raw.txt`, `data/perf_raw_demo_b6.txt` | Raw `--perf-frames` lines (test room, and demo beat 6). |
| `SHA256SUMS` | Checksums. |

The frames were captured on the branch rebased onto the merged HUD and party work (hence the Carl and Donut plates). The camera is stepped once per simulated tick, also inside `--capture-sequence` (it steps six ticks between frames and calls the same `camera_tick` after each), so the strip and the videos are samples of the same per-tick camera as the traces. A `--capture-tick` still shows the camera framed at once at that tick, not the eased history. Provenance: frames from `src/bin/floor01_scene.rs` of the commit that carries this directory, `xvfb-run` at 1920x1080 and 1280x800 (software-presented, the Vulkan adapter is the one of the CI runner class; used for pictures only, never for timing), scale 3 and 2, A1 lighting on unless stated. Regenerate:

```sh
bash scripts/export_assets.sh
cargo build --locked --bin floor01_scene
xvfb-run -a -s '-screen 0 1920x1080x24' ./target/debug/floor01_scene --scale 3 --fixture cp01_haymaker_chain \
  --camera-test-room 1 --camera auto --camera-test-threat 880:30:150 --capture-sequence 0:6:60 --capture-dir out/auto
./target/debug/floor01_scene --headless-floor01-smoke --fixture cp01_haymaker_chain --camera-test-room 1 \
  --camera auto --camera-test-threat 880:30:150 --capture-tick 420 --scale 3 --view-trace
```

Models: the code, the evidence and the documents were written by Claude Sonnet 5.5 (the medium tier). This is a Bevy integration PR, not an art PR: nothing here is new art, so the art pipeline (design, execution, independent art review) does not apply; the independent code review is separate. The images are published under the same owner-reported `ART-PUBLICATION-ALL-V1` scope as the source art and are not a licence to distribute a game.

## Results

Run: `cp01_haymaker_chain`, 420 ticks, synthetic foe aiming from tick 30 to 150, room 1584x768.

| Output | Rest widths | Widest | Ticks on a rest width | Largest width change per tick | Largest pan per tick | View inside the room |
|---|---|---|---|---|---|---|
| 1920x1080 | 640, 960 | 960 | 323 of 420 | 14 px | 10.0 px | yes (left 0, right 1120 of 1584) |
| 3840x2160 | 640, 768, 960 | 768 (the smallest integer-scale width that holds the foe) | 360 of 420 | 10 px | 10.0 px | yes |
| 1280x800 (Deck) | 640 | 640 | 420 of 420 | 0 | 4.0 px | yes |
| scroll camera | 640 | 640 | 420 of 420 | 0 | 3.0 px | yes |

The limits are the lab's (at most 14 px of width and 12.5 px of pan per tick). The ticks off a rest width are the two eased transits (out about 70 ticks, back about 70 ticks); a longer run has a larger share on rest widths (the lab's histograms: 95 percent or more).

**Same hashes with any camera.** `cp01_haymaker_chain`, `cp03_goblin_pileup` and `cp01_kick` on the test room with the synthetic foe give the same `FLOOR1_SCENE_STATE` and the same combat hash with `--camera scroll`, `auto`, `1.0`, `1.25` and `1.5` (md5 of the two lines: `bb01e9c697f0`, `6e201c5f5dcc`, `631f71a35a88` for all five). `bash scripts/ci_floor01_combat.sh headless` still replays every committed fixture log, gore preset, and animation trace byte for byte, and now repeats the camera check (`CAMERA_HASH_OK`) for two fixtures on the test room, one on the corridor and `--demo --demo-beat 4` on the hall (scroll, auto and 1.5). The unit test `scene_camera::tests::the_replay_hash_is_identical_with_the_camera_on_and_off` does it in the library.

**The lighting pass at every zoom width.** The A1 pass needed no change. The slot targets are allocated at 960x540 (the widest view), each frame the slot cameras draw the current view 1:1 into their top-left region and the composite pass lights that region at output resolution (the lab's approach). Captured at 640 (rest), through the transit (the strip: 642 to 958 px wide frames) and at 960 (rest), lit and unlit, at 1920x1080. No `FLOOR1_LIGHTING_WARNING` (the 64 light budget holds in a 960 px view of the test room). Nothing was fixed at 640 and no zoom was faked.

**The CP-01 camera kick at 1.0 and 1.5.** The kick (a whole-pixel offset of the left edge, opposite the hit) is scaled by the view width, so it moves the picture by the same number of output pixels at both. Largest kick of the three fixtures, from `--view-trace` at 1080p:

| Fixture | Camera | View width | Kick world px (scroll) | Kick drawn, world px | Kick on screen, output px |
|---|---|---|---|---|---|
| cp01_single_goblin | 1.0 | 640 | -3 | 3 | 9 |
| cp01_single_goblin | 1.5 | 960 | -3 | 5 | 10 |
| cp01_haymaker_chain | 1.0 | 640 | -3 | 3 | 9 |
| cp01_haymaker_chain | 1.5 | 960 | -3 | 5 | 10 |
| cp01_kick | 1.0 | 640 | -3 | 3 | 9 |
| cp01_kick | 1.5 | 960 | -3 | 5 | 10 |

(The largest kick of these fixtures is 3 world px; 3 px at 960 wide would be 6 output px, 5 px is 10, so it reads at least as strongly as at 1.0. Whether it *feels* right is for the owner.)

## Performance

Measured on the development machine's real display (an NVIDIA RTX 4060 Ti, Vulkan), `--release`, on the branch rebased onto the HUD, the party work and the demo hall, `--perf-frames 300 --perf-from-tick 0` (one fixture tick per measured frame after 120 warm-up frames, no vsync), two runs per row. **One development machine, a shared desktop; not a Steam Deck measurement and no Steam Deck claim.**

Test room, `cp01_haymaker_chain` with the synthetic foe (5356 sprites, lit), so the 300 frames include the widen to 960, the hold and the return ([raw](data/perf_raw.txt)):

| Output | Camera | Median frame | p95 | Max | GPU passes, mean | Resident memory |
|---|---|---|---|---|---|---|
| 1920x1080 | scroll | 1.82 to 2.03 ms | 2.4 ms | 4.5 to 4.9 ms | 0.56 ms | 398 to 410 MiB |
| 1920x1080 | auto | 2.28 to 2.51 ms | 2.8 to 3.6 ms | 4.9 to 5.1 ms | 0.73 ms | 395 to 420 MiB |
| 3840x2160 | scroll | 2.35 ms | 3.0 to 3.3 ms | 6.5 ms | 1.80 ms | 442 to 445 MiB |
| 3840x2160 | auto | 2.70 to 2.89 ms | 3.8 to 4.1 ms | 7.7 to 7.9 ms | 2.12 to 2.18 ms | 472 to 483 MiB |

Demo hall, beat 6 (the pileup, the plan's B6; 9918 sprites; no foe is in range there, so the automatic camera stays at 640 and the rows show the cost of the larger slot targets only) ([raw](data/perf_raw_demo_b6.txt)):

| Output | Camera | Median frame | p95 | Max | GPU passes, mean | Resident memory |
|---|---|---|---|---|---|---|
| 1920x1080 | scroll | 1.60 to 2.05 ms | 2.3 to 2.4 ms | 4.2 to 4.8 ms | 0.62 ms | 410 to 412 MiB |
| 1920x1080 | auto | 2.03 to 2.30 ms | 2.8 to 3.0 ms | 4.5 to 5.3 ms | 0.68 ms | 400 to 424 MiB |
| 3840x2160 | scroll | 2.69 to 2.75 ms | 3.2 to 5.7 ms | 7.7 to 9.8 ms | 2.07 to 2.15 ms | 450 to 477 MiB |
| 3840x2160 | auto | 2.86 to 2.92 ms | 3.8 to 4.7 ms | 8.4 to 9.3 ms | 2.29 to 2.32 ms | 481 to 495 MiB |

The automatic camera adds 0.06 to 0.17 ms of GPU time at 1080p and 0.2 to 0.35 ms at 4K (the composite pass lights a wider region while the view is at 960, and the post passes shade the larger slot targets), and the median frame grows by 0.1 to 0.5 ms; every row stays at or below 3 ms median and 5 ms p95 apart from one 5.7 ms p95. Resident memory grows by up to about 40 MiB (larger slot targets), within a spread of about 30 MiB between runs. The B4 beat (the llama) does not exist yet; its figures wait for PR B.

## Known limits and not done, on purpose

- The real llama (PR B) feeds `CameraInterest`; until then the zoom is demonstrated with the synthetic foe only, and the plan's "B4 llama widens to 960 and returns" capture waits for PR B.
- **HUD clearance at 960 (playtest item).** The operator keeps the lab's floor share, so the walking line sits at 288, 297 and 310 px of the HUD's 360 px layout space at rest widths 640, 768 and 960 (unit-tested in `scene_camera::tests`). The HUD notice box starts at 320, so at 960 a notice can sit about 10 px under the actors' feet. The hall frames show the plates and the actors at 960. Not changed; for the owner and the HUD owner to judge.
- Not run: 4K captures (only 4K traces), the 2P leash, Steam Deck option C, a human playtest.
