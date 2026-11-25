# DR-A evidence: room 3, the Dozer revenge ("The Machine Remembers")

Real windowed Bevy captures (`floor01_scene`, 640 x 360 and 640 x 400 virtual frames) of `--scene f01_late --f01-room dozer` under the story loadout: Carl under the pilot, Donut the autonomous `cf02_free` AI partner (`COMPANION_CONFIG: set=cf02_free`, the mode a player of the room sees). The machine is drawn with the approved **DOZ-01** rig (hull, tread, riders, front cab and tub, spiked wheel, yoke, smoke and steam puffs; [`../doz-01`](../doz-01/README.md)); there is **no box stand-in** for it. The rear gate, the buffer stop and the damage marker are code-drawn **labelled stand-ins** with a `STAND-IN` tag until DOZ-02 draws them (DZ-AD13). Nothing here approves tuning, art or game feel.

The input is the room pilot `dozer:close_call` (`src/room_pilots.rs`, `dozer_escape`): Carl walks to x 480, holds there until the wheel is 64 px away, then runs to the exit. Carl's input is a function of the simulation state only (DRA-AD2: stateless). The pilot is **aligned** (DRA-AD5, the #282 review F1): the party is the `cf02_free` partner's, so Donut is driven by the AI partner, not by the legacy follower; the room takes no `--cf02-seed` (the aligned seed 0 is nominal), like a player's run. Every capture is reproducible.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace && bash scripts/export_assets.sh
bash scripts/make_dozer_revenge_evidence.sh   # needs xvfb-run, ImageMagick and ffmpeg; ~6 minutes
python3 scripts/check_docs.py                 # verifies SHA256SUMS
```

`perf.txt` is made on its own with `scripts/room_perf.sh dozer dozer:close_call 499` (three runs per view, plus `--deck` for 640 x 400) and the `F01_DOZER_NAV` line of the headless room smoke (`--f01-room dozer --room-pilot <plan> --exit-after-ticks 520`).

## Stills (tick of the `dozer:close_call` run)

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `trigger_*.png` | 30 | The rumble: Carl has crossed the trigger, the rear gate stand-in shakes and smoke rises over it; the machine is not drawn yet (it is still behind the gate). |
| `chase_*.png` | 200 | The committed chase (cruise band): the machine rolls right, wheel and tread turning by distance, the riders on the hull; Carl holds at x 480 with Donut beside him. |
| `close_call_*.png` | 242 | The mercy band and the close call: the wheel is a few px from Carl, the riders yell (within 120 px), Carl starts to run. |
| `run_*.png` | 306 | Cruise again: Carl and the AI partner Donut run ahead of the machine. |
| `brake_*.png` | 476 | The brake at the buffer stop stand-in: the riders lurch forward, the machine slides 4 px. |
| `exit_*.png` | 498 | Carl reaches the exit under the lintel; the machine strains at the buffer stop (the engineer holds the lever, the riders jeer). |

Each still exists at `640x360` (`--scale 1`) and `640x400` (`--deck --scale 1`). `chase_640x360.gif` is ticks 174 to 498, every 6th tick (55 frames, 10 frames a second, 0.6x game speed, 2.4 MB).

Timeline of the run (the headless `COMBAT_EVENT` lines): rumble 21, emerged 111, bands catch_up, cruise from 132, mercy from 237, cruise from 297, brake 473, escaped 498. Without a pilot (the smoke's walk right) the exit is reached on tick 344. Every `DOZER_STILL` and `DOZER_GIF` line carries `partner=cf02_free`, read from the capture's own `COMPANION_CONFIG` line; the script fails on any other value.

## Read-only presentation

- `capture_lines.txt`: one `DOZER_STILL` line per still and size, one `DOZER_GIF` line. Every capture ran in the window under `xvfb-run` and headless to the same last tick: `window_hash == headless_hash` (`hash_match=yes`) for all 12 stills and the GIF.
- Every still was captured in two full passes; the PNGs of the two passes are byte-identical (`passes_byte_identical=yes`).
- The unit test `dozer_art::tests::drawing_never_changes_the_state_hash` runs the room with and without calling the presenter every tick and compares the state hash on every tick.

## PLAYTRACE session

`playtrace_session.txt`: one session of the same pilot recorded in the real window (`--record --exit-after-ticks 520`), verified with the late-Floor-1 replay rebuild: `PLAYTRACE_VERIFY result=PASS scope=full ticks=520 ... build=identical`, `state_hashes=520 event_hashes=520`. `effective_inputs=520` and the `COMPANION_CONFIG: set=cf02_free` line: the aligned pilot's inputs are recorded and replayed through the partner's party (the replay rebuilds `live_tuning` from the manifest flag `companion=cf02_free`). Session ids and build ids are stripped.

## Frame time and nav rebuilds (`perf.txt`)

Debug build, xvfb (not a real display), AMD Ryzen 9 5950X, NVIDIA GeForce RTX 4060 Ti, 499 frames a run (the exit tick 498 plus one), 120 warm-up frames, the `cf02_free` partner active in every run (a `# COMPANION_CONFIG: set=cf02_free` line follows each run in `perf.txt`), load average below 2.5 during the runs:

| View | Median frame (3 runs) | p95 frame | Sim tick mean / p95 | GPU mean | Entities |
| --- | --- | --- | --- | --- | --- |
| 640 x 360 | 16.21 to 16.32 ms | 20.06 to 20.49 ms | 61.2 to 62.5 us / 308.6 to 314.0 us | 1.74 to 1.79 ms | 5715 |
| 640 x 400 | 16.38 to 16.44 ms | 19.73 to 20.00 ms | 60.2 to 63.9 us / 306.7 to 310.4 us | 1.76 to 1.80 ms | 5715 |

The frame interval is the xvfb present pace, not a real-display frame time. The simulation tick costs about 62 us on average and about 310 us at p95: the AI partner's planning runs every tick (the earlier measurement, about 17 us, ran the follower by mistake: the #282 review F1).

**Nav rebuild count, with the partner active.** The `cf02_free` partner's hazard feed (the wheel's nav rows) changed **63** times in `dozer:run` (344 ticks), **62** in `dozer:close_call` and **62** in `dozer:wait` (498 ticks each), with `hazard_feed_errors=0`, `wheel_hits=0` and no rescue (`F01_DOZER_NAV` lines in `perf.txt`). The machine is a hazard the partner's nav reads; the rows are filled into a reused buffer (`Dozer::nav_rows_into`), so a tick allocates nothing for them. The no-pilot smoke reads 0 (`COMPANION_CONFIG: set=cf02_free inactive`): its scripted walk runs the legacy follower, not the partner, and is a CI smoke, not the mode a player sees. The earlier claim of this README (0 rebuilds, "CA2-FU-HAZ U-1 stays open") was measured with the follower by mistake and was wrong.

## NOT RUN and limits

- **Human pad evaluation: NOT RUN.** Game feel is the owner's; nothing here was played on a pad.
- **Real-display frame time and release build: NOT RUN.** The numbers above are debug under xvfb.
- **Two players, solo Donut, the wipe and retry, damage states (DR-B), the corral and later rooms (DR-C, DR-D): NOT RUN** (out of DR-A's scope). The damage marker stand-in is unreachable before DR-B.
- The pilots are not players: they never misjudge the gap.

## Rights and provenance

- DOZ-01 rig: designed by Claude Opus, size decided by a separate Claude Opus agent, executed by Claude Sonnet, reviewed by independent Claude Opus agents and approved by the owner ([`../doz-01`](../doz-01/README.md)); the riders reuse GOB-01 goblin pixels.
- DR-A wiring, the labelled stand-ins and these captures: Claude Opus, as assigned by the orchestrator's brief.
- Original fan-concept art published under the owner-reported `ART-PUBLICATION-ALL-V1` scope (`docs/04-production/rights-and-release.md`). No third-party assets.
