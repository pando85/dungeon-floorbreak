# DR-A and DR-B evidence: room 3, the Dozer revenge ("The Machine Remembers")

Real windowed Bevy captures (`floor01_scene`, 640 x 360 and 640 x 400 virtual frames) of `--scene f01_late --f01-room dozer` under the story loadout: Carl under the pilot, Donut the autonomous `cf02_free` AI partner (`COMPANION_CONFIG: set=cf02_free`, the mode a player of the room sees). The machine is drawn with the approved **DOZ-01** rig (hull, tread, riders, front cab and tub, spiked wheel, yoke, smoke and steam puffs; [`../doz-01`](../doz-01/README.md)); there is **no box stand-in** for it. The rear gate, the buffer stop and the damage marker are code-drawn **labelled stand-ins** with a `STAND-IN` tag until DOZ-02 draws them (DZ-AD13); since DR-B so are the stop frame, its chain, its floor shadow, the brace (intact, weakened, snapped), the remnant and the rear glance. Nothing here approves tuning, art or game feel.

The input is the room pilot `dozer:close_call` (`src/room_pilots.rs`, `dozer_escape`): Carl walks to x 480, holds there until the wheel is 64 px away, then runs to the exit. Carl's input is a function of the simulation state only (DRA-AD2: stateless). The pilot is **aligned** (DRA-AD5, the #282 review F1): the party is the `cf02_free` partner's, so Donut is driven by the AI partner, not by the legacy follower; the room takes no `--cf02-seed` (the aligned seed 0 is nominal), like a player's run. Every capture is reproducible.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace && bash scripts/export_assets.sh
bash scripts/make_dozer_revenge_evidence.sh   # needs xvfb-run, ImageMagick and ffmpeg; ~6 minutes
python3 scripts/check_docs.py                 # verifies SHA256SUMS
```

`perf.txt` is made on its own with `scripts/room_perf.sh dozer dozer:close_call 478` (three runs per view, plus `--deck` for 640 x 400) and the `F01_DOZER_NAV` line of the headless room smoke (`--f01-room dozer --room-pilot <plan> --exit-after-ticks 520`).

## Stills (tick of the `dozer:close_call` run)

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `trigger_*.png` | 30 | The rumble: Carl has crossed the trigger, the rear gate stand-in shakes and smoke rises over it; the machine is not drawn yet (it is still behind the gate). |
| `chase_*.png` | 200 | The committed chase (cruise band): the machine rolls right, wheel and tread turning by distance, the riders on the hull; Carl holds at x 480 with Donut beside him. |
| `close_call_*.png` | 242 | The mercy band and the close call: the wheel is a few px from Carl, the riders yell (within 120 px), Carl starts to run. |
| `run_*.png` | 306 | Cruise again: Carl and the AI partner Donut run ahead of the machine. |
| `brake_*.png` | 474 | The brake at the buffer stop stand-in: the riders lurch forward, the machine slides 4 px. |
| `exit_*.png` | 477 | The room completes: the AI partner Donut, who ran ahead since DR-B, reaches the exit under the lintel first (DV-8); Carl is behind her and the machine brakes at the buffer stop. |

Each still exists at `640x360` (`--scale 1`) and `640x400` (`--deck --scale 1`). `chase_640x360.gif` is ticks 174 to 474, every 6th tick (51 frames, 10 frames a second, 0.6x game speed).

Timeline of the run (the headless `COMBAT_EVENT` lines): rumble 21, emerged 111, bands catch_up, cruise from 132, mercy from 237, cruise from 297, brake 473, and the room completes on 477 when Donut reaches the exit (before DR-B Carl reached it first, on 498: with the frame's prop and hazard in the room the `cf02_free` partner takes a different line and runs ahead; the machine's own timeline is unchanged). Without a pilot (the smoke's walk right) the exit is reached on tick 344. Every `DOZER_STILL` and `DOZER_GIF` line carries `partner=cf02_free`, read from the capture's own `COMPANION_CONFIG` line; the script fails on any other value.

## DR-B: the reversal (`dozer:brace`)

The room pilot `dozer:brace` (`src/room_pilots.rs`, `dozer_brace`) is the live twin of the `dz_frame_disable_once` fixture: Carl walks to x 856, turns back to the brace at x 832 (so he faces it), and on the machine's first committed-chase tick aims the release at the middle of the helper's useful window (`dozer::release_window` for a hero at the brace, read from the live frame: R 163 to 246, so R 204). He jabs the brace twice, holds while the frame lands on the wheel front, and runs once the machine is stunned. Donut is the autonomous `cf02_free` partner, as in the escape plans. The pilot keeps the first chase tick, so it is aligned only.

Timeline (the headless `COMBAT_EVENT` lines): emerged 111, the jabs hit the brace on 303 (weakened) and 315 (released, R 204, credited to player 0), the frame falls from 344 and lands on 352 on the front collider with the wheel front at x 728 (`impact_stunned`, `by=0`), the machine is `disabled` on 400 (the 48-tick stun), and the room completes on 490. Nobody is hurt and the wheel hits nobody (the unit test `the_dozer_brace_pilot_reverses_the_chase`).

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `brace_release_*.png` | 318 | The warning: the brace stand-in snapped, the frame shakes on its chain, its shadow on the floor; Carl at the brace, the machine coming. |
| `brace_fall_*.png` | 348 | The frame falls (no chain) onto the footprint; the wheel front is under it. |
| `brace_stun_*.png` | 353 | The landing on the front: the machine is stunned (the damage marker stand-in), the remnant lies under the jammed front. |
| `brace_disabled_*.png` | 404 | The machine is disabled; Carl runs for the exit. |
| `brace_run_*.png` | 440 | Carl and Donut run past the dead machine to the exit. |

`reversal_640x360.gif` is ticks 297 to 489, every 4th tick (49 frames, 10 frames a second, 0.4x game speed): the second jab, the warning, the fall, the stun, the disable and the run.

## Read-only presentation

- `capture_lines.txt`: one `DOZER_STILL` line per still and size, one `DOZER_GIF` line per GIF. Every capture ran in the window under `xvfb-run` and headless to the same last tick: `window_hash == headless_hash` (`hash_match=yes`) for all 22 stills and both GIFs.
- Every still was captured in two full passes; the PNGs of the two passes are byte-identical (`passes_byte_identical=yes`).
- The unit test `dozer_art::tests::drawing_never_changes_the_state_hash` runs the room with and without calling the presenter every tick and compares the state hash on every tick; `the_frame_stand_ins_follow_the_frame_and_the_rear_glance` (DR-B) does the same through a release, a landing and a rear glance, and checks every frame, brace, shadow, remnant and glance label.

## PLAYTRACE session

`playtrace_session.txt`: one session of the same pilot recorded in the real window (`--record --exit-after-ticks 520`), verified with the late-Floor-1 replay rebuild: `PLAYTRACE_VERIFY result=PASS scope=full ticks=520 ... build=identical`, `state_hashes=520 event_hashes=520`. `effective_inputs=520` and the `COMPANION_CONFIG: set=cf02_free` line: the aligned pilot's inputs are recorded and replayed through the partner's party (the replay rebuilds `live_tuning` from the manifest flag `companion=cf02_free`). Session ids and build ids are stripped.

## Frame time and nav rebuilds (`perf.txt`)

Debug build, xvfb (not a real display), AMD Ryzen 9 5950X, NVIDIA GeForce RTX 4060 Ti, 478 frames a run (the exit tick 477 plus one), 120 warm-up frames, the `cf02_free` partner active in every run (a `# COMPANION_CONFIG: set=cf02_free` line follows each run in `perf.txt`), load average 2.3 to 4.8 after the runs (`loadavg_after` in `perf.txt`; the machine was shared with other builds):

| View | Median frame (3 runs) | p95 frame | Sim tick mean / p95 | GPU mean | Entities |
| --- | --- | --- | --- | --- | --- |
| 640 x 360 | 16.32 to 16.37 ms | 19.82 to 19.98 ms | 81.0 to 88.1 us / 429.0 to 439.4 us | 1.66 to 1.78 ms | 5544 |
| 640 x 400 | 16.30 to 17.52 ms | 19.99 to 24.47 ms | 94.2 to 127.5 us / 438.9 to 618.6 us | 1.74 to 1.84 ms | 5544 |

The frame interval is the xvfb present pace, not a real-display frame time. The simulation tick's p95 is the cost of the partner's nav rebuilds, not of per-tick planning: 62 of the 478 ticks of a run (13 %) rebuild the partner's nav graph from the wheel's hazard rows, so the 95th percentile falls on a rebuild tick (about 0.4 to 0.6 ms each in this run), while the mean (81 to 128 us here) is the ordinary tick. The #282 re-review read about 63 us at p95 for the partner-active mastery arena, which has no moving hazard rows. The DR-A run of this table (load average below 2.5) read about 62 us mean and 310 us p95; this DR-B run is higher on both under a higher load average (the first 640 x 400 run, the noisiest, at load 4.3), and the difference was not separated into load and the frame's added prop and hazard (a debug build under xvfb is no benchmark). The earlier measurement of DR-A, about 17 us, ran the follower by mistake (the #282 review F1).

**Nav rebuild count, with the partner active.** The `cf02_free` partner's hazard feed (the wheel's nav rows) changed **63** times in `dozer:run` (344 ticks), **62** in `dozer:close_call` (477 ticks), **62** in `dozer:wait` (500 ticks) and **46** in `dozer:brace` (490 ticks: the machine stops on 352), with `hazard_feed_errors=0`, `wheel_hits=0` and no rescue (`F01_DOZER_NAV` lines in `perf.txt`). The machine is a hazard the partner's nav reads; the rows are filled into a reused buffer (`Dozer::nav_rows_into`), so a tick allocates nothing for them. The no-pilot smoke reads 0 (`COMPANION_CONFIG: set=cf02_free inactive`): its scripted walk runs the legacy follower, not the partner, and is a CI smoke, not the mode a player sees. The unit test `the_dozer_pilots_escape_room_3` asserts the count of every dozer pilot is above 0 (the #282 review R2). The earlier claim of this README (0 rebuilds, "CA2-FU-HAZ U-1 stays open") was measured with the follower by mistake and was wrong.

## NOT RUN and limits

- **Human pad evaluation: NOT RUN.** Game feel is the owner's; nothing here was played on a pad.
- **Real-display frame time and release build: NOT RUN.** The numbers above are debug under xvfb.
- **Two players, solo Donut, the wipe and retry in the window, the crew and later rooms (DR-C, DR-D): NOT RUN** (out of scope). The two-pad and Donut-missile releases are fixtures (`dz_frame_missile`, `dz_frame_dynamite`), not captures.
- The pilots are not players: they never misjudge the gap.

## Rights and provenance

- DOZ-01 rig: designed by Claude Opus, size decided by a separate Claude Opus agent, executed by Claude Sonnet, reviewed by independent Claude Opus agents and approved by the owner ([`../doz-01`](../doz-01/README.md)); the riders reuse GOB-01 goblin pixels.
- DR-A wiring, the labelled stand-ins and these captures: Claude Opus, as assigned by the orchestrator's brief. DR-B (the frame, the brace and glance stand-ins, the `dozer:brace` pilot and the reversal captures): Claude Opus, as assigned by the orchestrator's brief; the stand-ins are code-drawn rectangles, no new art.
- Original fan-concept art published under the owner-reported `ART-PUBLICATION-ALL-V1` scope (`docs/04-production/rights-and-release.md`). No third-party assets.
