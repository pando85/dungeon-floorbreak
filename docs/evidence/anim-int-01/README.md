# ANIM-INT-01 evidence: locomotion animation in the Floor 1 scene

Real Bevy captures (not Pillow models) of the Floor 1 corridor scene playing the owner-approved Carl and Donut locomotion atlases, with the A1 lighting pass on, at 1920x1080 (3x of 640x360). Design and rules: [scene doc, Animation](../../03-tech/floor01-scene.md#animation-anim-int-01). Art: [Carl](../../02-art/carl-locomotion-anim-01.md), [Donut](../../02-art/donut-locomotion-anim-01.md). Status: functional integration for review; **no human has judged game feel**. The frames come from a software Vulkan renderer under Xvfb on a developer machine; CI repeats the capture and uploads its own set as the `the-crawl-floor01-scene` artifact.

- **Rights record:** `ART-PUBLICATION-ALL-V1` in [`docs/04-production/rights-and-release.md`](../../04-production/rights-and-release.md) (owner-reported publication scope, not independently verified; no public reuse licence and no game or standalone asset-pack distribution right).
- **Integrity:** `SHA256SUMS` lists every file except this README; `python3 scripts/check_docs.py` verifies it.
- **Provenance:** frames captured by `floor01_scene --scenario run_jump --capture-sequence`; GIF, stills and timeline built by `scripts/make_anim_int_evidence.py` (Pillow). Integration code and evidence tooling: Claude Sonnet.

| File | Content |
| --- | --- |
| `run_jump_1080p.gif` | the whole run, 81 frames, one frame per 3 fixed ticks (50 ms, so it plays in real time), cropped to 1056 x 594 of the 1920x1080 window around the actors, with the tick and both actors' `tag:atlas frame` burned into the top bar |
| `still_mid_run.png` | full 1920x1080 frame, tick 160: Carl `run:8`, Donut `run:7`, both on the floor |
| `still_jump_start.png` | tick 172: the first captured tick after the impulse, Carl `jump_start:11` (the toe-off frame, already rising) and Donut still running |
| `strip_takeoff.png` | **every tick** (168 to 182, no sampling) around the running takeoff: run, the impulse tick 171 with `jump_start:11`, hold, `jump_rise:12` from tick 176; no crouch in the air, no pop |
| `strip_landing.png` | **every tick** (218 to 230) around the running landing: `land:14` on ticks 220 to 224, `run:4` on tick 225 (the cut), `land:15` never shown |
| `still_apex.png` | tick 196, the apex tick of the running jump: Carl switches to `jump_fall:13`, Donut is still rising in `jump_rise:12` (she replays his path 10 ticks later) |
| `still_landing.png` | tick 220, the touchdown tick: Carl `land:14` (impact) with the feet on the floor, Donut still falling |
| `timeline_run_jump.png` | **for the owner:** ticks on the x axis (ticks 142 to 382), one bar per atlas frame with its index for Carl and Donut, colour per state, foot-height curves of both actors, markers for the jump input, the apex and the touchdown (Carl above, Donut below) |
| `anim_trace.txt` | one `FLOOR1_ANIM_TRACE` row per simulated tick (0 to 382) from the headless run: hero position, velocity, grounded, and what each animator shows, plus the final state lines |
| `capture_log.txt`, `capture_log_strip_takeoff.txt`, `capture_log_strip_landing.txt` | the `FLOOR1_CAPTURE`, `FLOOR1_SCENE_STATE` and `FLOOR1_ANIM_STATE` lines of the 81 GIF frames and of the two strips |
| `SHA256SUMS` | checksums |

## The scenario (`--scenario run_jump`)

Stand until tick 30, run right (the `walk_right` rule jumps the two low steps near the spawn), **running jump** at tick 170 on the open floor, keep running (the landing is cut into the run), stop at tick 260, **standing jump** at tick 290 and stand through the landing. The capture covers ticks 142 to 382 every 3 ticks.

| Event | Tick | Carl | Donut |
| --- | --- | --- | --- |
| jump input applied (impulse tick) | 171 | `jump_start:11` on the same tick, `jump_rise:12` from tick 176 | still running |
| Donut leaves the ground (32 px later at 185 px/s) | 181 | `jump_rise:12` | `jump_start:11`, `jump_rise:12` from tick 186 |
| apex (first tick with `vy <= 0`) | 196 | `jump_fall:13` | `jump_rise:12` |
| Donut's apex | 206 | `jump_fall:13` | `jump_fall:13` |
| touchdown | 220 | `land:14` (impact, 5 ticks), `run:4` on tick 225 | `jump_fall:13` |
| Donut touches down | 231 | `run` | `land:14`, run again on tick 236 |
| stop (stick released) | 261 | `idle:0` at once | `idle:0` on the same tick (her trail stops; no hold, no hover on the gallop frame) |
| standing jump: impulse / apex / touchdown | 291 / 316 / 340 | `jump_start:11` / `jump_fall:13` / `land:14`, idle from tick 351 | idle throughout (her trail does not move) |

## What was checked

- **The simulation did not change.** `FLOOR1_SCENE_STATE` of `walk_right` at ticks 250 and 613, `turn_around` at 300 and `idle` at 100 are identical to the pre-change binary (hashes `931868d6afa56498`, `82b8f87fa644de48`, `75255221e2d2c9e9`, `fd9659435d2907c8`), the existing golden traces pass unchanged, and a unit test compares the state hash on every tick with and without animation. The `--reference-frame` capture is pixel-identical (0 differing pixels) to the pre-change binary, and the Pillow comparison gates still pass.
- **Takeoff and landing, tick by tick.** The two strips show every tick. After the first review round the crouch frame is no longer shown in the air, the recovery frame no longer flashes for one tick when Carl lands running, and Donut no longer flashes idle on one-tick trail stalls on a paw frame (unit tests pin run_jump t41 and walk_right t11); at a real stop she goes idle on the same tick (run_jump t261).
- **Render and headless agree.** For all 81 captures the animation state the render printed equals the state of the headless trace at that tick (`make_anim_int_evidence.py` refuses to build otherwise), and CI requires the committed `anim_trace.txt` and `capture_log.txt` to be reproduced exactly.
- **Frame times.** Carl runs at 43 ms per frame at 185 px/s (slip-free), Donut at 80/80/80/80/90/80 ms; see the scene doc for the formulas and constants.

## Regenerate

```bash
bash scripts/export_assets.sh
cargo build --locked --bin floor01_scene
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_render_smoke.sh anim-sequence   # frames + log in artifacts/
python3 scripts/make_anim_int_evidence.py --frames artifacts/anim-int-01/frames \
  --log artifacts/floor01-scene-anim-sequence-1920x1080.log \
  --trace artifacts/anim-int-01/anim_trace_full.txt --out docs/evidence/anim-int-01
```

The state logs are deterministic; the PNG and GIF pixels depend on the GPU and driver, so they are not compared bit for bit across machines.

## Known gaps

- **Donut's hop onto the low platform (ticks 91 to 96 of the scenario, not in the GIF window) is still rough.** The simulation's trail collapses vertical-only samples, so her replayed foot height rises about 19 px in one tick (92, 96.9, 116.1, 122.1, 127.8, 133.2) above the 108 px platform top and snaps to 108 on touchdown. After the takeoff fix she holds `jump_start:11` for the whole hop instead of showing the crouch, which reads better, but the vertical snap remains. The trail is simulation code and is unchanged in this PR (see the scene doc, known gaps).
- Donut's contact shadow keeps the old idle footprint and does not follow her stretched run frames (per-tag footprint deferred).
- A trail stall on Donut's gallop suspension frames (atlas 8 and 9) still flashes idle for a few ticks (walk_right t468-470, outside this scenario): holding them would hover her 4 px above the floor.
- Software-rendered frames say nothing about performance, GPU look or controller feel.
