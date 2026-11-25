# DR-A evidence: room 3, the Dozer revenge ("The Machine Remembers")

Real windowed Bevy captures (`floor01_scene`, 640 x 360 and 640 x 400 virtual frames) of `--scene f01_late --f01-room dozer` under the story loadout, solo Carl with the follower Donut. The machine is drawn with the approved **DOZ-01** rig (hull, tread, riders, front cab and tub, spiked wheel, yoke, smoke and steam puffs; [`../doz-01`](../doz-01/README.md)); there is **no box stand-in** for it. The rear gate, the buffer stop and the damage marker are code-drawn **labelled stand-ins** with a `STAND-IN` tag until DOZ-02 draws them (DZ-AD13). Nothing here approves tuning, art or game feel.

The input is the room pilot `dozer:close_call` (`src/room_pilots.rs`, `dozer_escape`): Carl walks to x 480, holds there until the wheel is 64 px away, then runs to the exit. It is a function of the simulation state only (DRA-AD2: stateless, no seed), so every capture is reproducible.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace && bash scripts/export_assets.sh
bash scripts/make_dozer_revenge_evidence.sh   # needs xvfb-run, ImageMagick and ffmpeg; ~6 minutes
python3 scripts/check_docs.py                 # verifies SHA256SUMS
```

`perf.txt` is made on its own with `scripts/room_perf.sh dozer dozer:close_call 493` (three runs per view, plus `--deck` for 640 x 400) and the `F01_DOZER_NAV` line of the headless room smoke (`--f01-room dozer --room-pilot <plan> --exit-after-ticks 520`).

## Stills (tick of the `dozer:close_call` run)

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `trigger_*.png` | 30 | The rumble: Carl has crossed the trigger, the rear gate stand-in shakes and smoke rises over it; the machine is not drawn yet (it is still behind the gate). |
| `chase_*.png` | 200 | The committed chase (cruise band): the machine rolls right, wheel and tread turning by distance, the riders on the hull; Carl holds at x 480. |
| `close_call_*.png` | 236 | The mercy band and the close call: the wheel is a few px from Carl, the riders yell (within 120 px), Carl starts to run. |
| `run_*.png` | 300 | Cruise again: Carl and Donut run ahead of the machine. |
| `brake_*.png` | 470 | The brake at the buffer stop stand-in: the riders lurch forward, the machine slides 4 px. |
| `exit_*.png` | 492 | Carl reaches the exit under the lintel; the machine strains at the buffer stop (the engineer holds the lever, the riders jeer). |

Each still exists at `640x360` (`--scale 1`) and `640x400` (`--deck --scale 1`). `chase_640x360.gif` is ticks 168 to 492, every 6th tick (55 frames, 10 frames a second, 0.6x game speed, 2.4 MB).

Timeline of the run (the headless `COMBAT_EVENT` lines): rumble 21, emerged 111, bands catch_up, cruise from 170, mercy from 231, cruise from 291, brake 467, escaped 492. Without a pilot (the smoke's walk right) the exit is reached on tick 344.

## Read-only presentation

- `capture_lines.txt`: one `DOZER_STILL` line per still and size, one `DOZER_GIF` line. Every capture ran in the window under `xvfb-run` and headless to the same last tick: `window_hash == headless_hash` (`hash_match=yes`) for all 12 stills and the GIF.
- Every still was captured in two full passes; the PNGs of the two passes are byte-identical (`passes_byte_identical=yes`).
- The unit test `dozer_art::tests::drawing_never_changes_the_state_hash` runs the room with and without calling the presenter every tick and compares the state hash on every tick.

## PLAYTRACE session

`playtrace_session.txt`: one session of the same pilot recorded in the real window (`--record --exit-after-ticks 520`), verified with the late-Floor-1 replay rebuild: `PLAYTRACE_VERIFY result=PASS scope=full ticks=520 ... build=identical`, `state_hashes=520 event_hashes=520`. `effective_inputs=0` is expected for a room-pilot session that is not aligned (the pilot is rebuilt from the state, not read back as inputs), as in the other committed room sessions. Session ids and build ids are stripped.

## Frame time and nav rebuilds (`perf.txt`)

Debug build, xvfb (not a real display), AMD Ryzen 9 5950X, NVIDIA GeForce RTX 4060 Ti, 493 frames a run, 120 warm-up frames:

| View | Median frame (3 runs) | p95 frame | Sim tick mean / p95 | GPU mean | Entities |
| --- | --- | --- | --- | --- | --- |
| 640 x 360 | 16.49 to 16.64 ms | 18.92 to 19.66 ms | 16.4 to 17.0 us / 25.5 to 27.0 us | 0.51 to 1.22 ms | 5715 |
| 640 x 400 | 16.45 to 16.55 ms | 19.12 to 19.39 ms | 15.8 to 18.0 us / 23.7 to 28.4 us | 0.59 to 1.34 ms | 5715 |

The frame interval is the xvfb present pace, not a real-display frame time; the simulation tick costs tens of microseconds.

**Nav rebuild count: 0.** The AI partner's hazard feed changed 0 times in `dozer:run`, `dozer:close_call`, `dozer:wait` and the no-pilot smoke (`F01_DOZER_NAV ... nav_rebuilds=0 hazard_feed_errors=0 wheel_hits=0`). The machine is not yet a hazard the partner's nav reads; CA2-FU-HAZ U-1 ("the Dozer wheel") stays open.

## NOT RUN and limits

- **Human pad evaluation: NOT RUN.** Game feel is the owner's; nothing here was played on a pad.
- **Real-display frame time and release build: NOT RUN.** The numbers above are debug under xvfb.
- **Two players, solo Donut, the wipe and retry, damage states (DR-B), the corral and later rooms (DR-C, DR-D): NOT RUN** (out of DR-A's scope). The damage marker stand-in is unreachable before DR-B.
- The pilots are not players: they never misjudge the gap.

## Rights and provenance

- DOZ-01 rig: designed by Claude Opus, size decided by a separate Claude Opus agent, executed by Claude Sonnet, reviewed by independent Claude Opus agents and approved by the owner ([`../doz-01`](../doz-01/README.md)); the riders reuse GOB-01 goblin pixels.
- DR-A wiring, the labelled stand-ins and these captures: Claude Opus, as assigned by the orchestrator's brief.
- Original fan-concept art published under the owner-reported `ART-PUBLICATION-ALL-V1` scope (`docs/04-production/rights-and-release.md`). No third-party assets.
