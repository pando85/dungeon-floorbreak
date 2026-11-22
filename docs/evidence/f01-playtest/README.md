# PT-GATE-A evidence: the gate tooling and the parts of the playtest gate that do not depend on PT-TUNE

Plan section 10 ([f01-playtest-integration-plan.md](../../07-agents/f01-playtest-integration-plan.md)), items 3, 4 and 7 in part. This is **not** the whole gate and it does not declare the build playtest-ready: room 6 (the mastery arena) and the owner packet (item 8) belong to PT-GATE-B, after PT-TUNE. Nothing here was played by a human, on a pad or on real hardware, and nothing is a game-feel verdict.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace && bash scripts/export_assets.sh   # needs xvfb-run and Pillow
bash scripts/make_f01_gate_a_perf.sh        # perf.txt: run it alone, on a quiet machine (wall-clock numbers)
bash scripts/make_f01_gate_a_evidence.sh    # grants, stills, recorded sessions; ~2 minutes
python3 scripts/check_docs.py               # verifies SHA256SUMS
```

## 1. Frame time (gate item 7), gore preset 4

`scripts/room_perf.sh ROOM PLAN FRAMES` runs the real Bevy window under `xvfb-run` at 640x360, 1x, lighting on, gore preset 4, `AutoNoVsync`, no audio. The room is driven by its room pilot (`--room-pilot`, `src/room_pilots.rs`): the room is pre-simulated to tick 0, then one pilot tick is stepped per measured frame after a 120-frame warm-up (`--perf-frames F --perf-from-tick 0`, solo pilot). `FLOOR1_PERF` now also prints `p99_ms`.

**The Boiler's full chain** (`bash scripts/room_perf.sh boiler chain 950`: lure, kick, valve, steam, brace, landing, drum, exit at tick 949; 950 frames; 844 death sprites spawned, 69 visible at peak, about 500 MiB resident). Three runs, because the interval is wall-clock:

| Run | median ms | p95 ms | p99 ms | max ms | mean ms | GPU ms mean | pilot tick us mean |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 16.24 | 31.32 | 38.49 | 48.54 | 17.87 | 1.23 | 42.3 |
| 2 | 16.57 | 19.09 | 21.30 | 61.88 | 16.69 | 1.24 | 35.4 |
| 3 | 16.42 | 20.41 | 26.25 | 38.36 | 16.74 | 1.03 | 42.6 |

**Machine and label.** AMD Ryzen 9 5950X (16 cores); wgpu reported the adapter as a discrete NVIDIA GeForce RTX 4060 Ti class GPU (Vulkan); dev profile (the CI profile); Linux with an Xvfb virtual display. **This is an xvfb run, not a real display or a real swapchain, and not a handheld**: the frame is copied through the X server, which paces the interval near 16.6 ms whatever the render cost, so the interval columns bound the xvfb path and say little about the game. The GPU column (summed timed render passes, about 1 ms) and the pilot's tick cost (tens of microseconds) are the informative ones. The software-render caveat of the task brief does not hold literally here (the adapter is the discrete GPU), but the presentation path is still virtual, so the numbers are labelled xvfb and not hardware. Run 1 had a busy neighbour (load average 4.4 after the run); each `ROOM_PERF` line in `perf.txt` carries the load average.

**The mastery arena (room 6): NOT RUN in this PR.** PT-TUNE changed its waves, so its number belongs to PT-GATE-B on the tuned room. The exact command, valid now that the `mastery` plan exists: `bash scripts/room_perf.sh mastery mastery 3000`.

**NOT RUN:** frame time on real hardware (a real swapchain, a Steam Deck, a 1080p window, release profile) and the 640x400 view.

## 2. GIF byte stability (a #233 re-review follow-up)

Cause (measured): the first screenshot of a `--capture-sequence` was taken 4 frames after the loading report, before the scene had rendered. In five of six xvfb runs frame 0 came out near-black (the HUD text over a dark frame, 102 colours) and in one fully rendered (5252 colours); `scripts/room_gif.py` samples its 96-colour palette from frame 0 among others, so the palette, and every byte of the GIF, varied. Fix: `CAPTURE_FIRST_SETTLE_FRAMES = 60` (10, 30 and 60 frames each gave the rendered frame in 6 of 6 runs), and `room_gif.py` now refuses a capture whose first frame is mostly black.

Regenerated: all 8 Boiler clips (`docs/evidence/f01-boiler`), 14 of the 16 CP-E GIFs (`docs/evidence/carl-prog-01/cp-e`; the two `reach-ring-outer-launch` GIFs came out byte-identical to the committed ones) and the hazard first-look GIF, with new `SHA256SUMS`. Every `capture_lines.txt`, `pilot_hashes.txt` and still is byte-identical to before, so no simulation hash and no golden moved. Proof: the three evidence scripts were run twice in a row on the final binary (the three in parallel, under load) and every output file of the second pass compared equal to the first (`diff -r`: identical).

## 3. The two grant moments (gate items 3 and 7)

| Moment | Source | Result |
| --- | --- | --- |
| The gauntlet (`fist_reinforced`) and the blink (`special_short_blink`) at the Juicer's defeat | `--scene f01_late --f01-room juicer --room-pilot juicer`: the PT-SCENE-B solo-Carl pilot (jab phase 0; a unit test ties it to `juicer_pilot`'s hash) under the story loadout | `grant-juicer-gauntlet-blink.gif` (76 frames from tick 3150, 100 ms each). The defeat and both `CapabilityGranted` events are tick 3175, one per grant (`grant_lines.txt`). |
| Dynamite (`special_thrown_charge`) | **The placeholder-zone fixture** `carl_prog_story_event_grants_dynamite` (`--party-fixture`), **not the Hoarder's room**: room 1 is not at HRD-B, so the gate allows the placeholder | `grant-dynamite-placeholder-zone.gif` (ticks 36 to 105): the grant is tick 69, the notice GOBLIN BOOM and the BOOM socket appear on it. |

What the frames show (looked at by the author, not an art verdict): in the Juicer GIF the boss bar empties and KNOCKOUT! shows; the System notice queue then shows the grant notices one after the other, HEAVY FIST (the gauntlet) and PUDDLE JUMPER (the blink), 136 ticks each (stills `still-juicer-notice-*`). **The pilot reaches the exit on tick 3255 and the room's simulation stops there, so every frame after it is a HUD tick only**: the scene is frozen with Carl and Donut at the gate while the notices play out. The dynamite still shows Carl crossing the zone under the arch with the BOOM socket lit.

**The PLAYTRACE journal** (section 4): the Juicer session holds exactly two `CapabilityGranted` lines, hero 0 `fist_reinforced` and hero 1 `special_short_blink`, both tick 3175, one per grant; the Boiler and hazard sessions hold none (the story loadout grants nothing there). **The dynamite journal check is NOT RUN as a live recording**: the placeholder fixture is a party fixture, which the recorder does not take (`--record` needs a windowed run of a room or scene), and the Hoarder room has no pilot that defeats her. Its one-receipt check is the library fixture `hrd_defeat_grants_dynamite` (`src/hoarder_fixtures.rs`).

## 4. One recorded session per room (gate item 4), strict

Recorded in the real window under xvfb with `--record` and a scripted room pilot, then verified in a fresh process with `playtrace verify` (`result=PASS scope=full`, `build=identical`). `playtrace_sessions.txt` has the lines.

| Room | Plan and length | Result |
| --- | --- | --- |
| 2 Boiler | `chain`, 960 ticks | PASS, 1 episode, 40 combat events |
| 4 hazards | `direct`, 1500 ticks | PASS, 4 episodes (the pilot is defeated and retries, it does not dodge hazards), 44 combat events |
| 5 Juicer | `juicer`, 3300 ticks | PASS, 1 episode, 216 combat events |

The sessions are **scripted-bot sessions** (`run_kind ScriptedBot`), not a human's: a **recorded human session per room is NOT RUN** (the owner's pad pass). The session files are not committed: the recording store is ignored on purpose and a verification needs the recording's exact source, so the evidence is the verification output and the script that reproduces it. **Room 6: NOT RUN** (PT-GATE-B, after PT-TUNE).

## NOT RUN in this PR

- Real-hardware frame time, a real two-pad 2P run, a human on a pad, a human-recorded session per room.
- Room 6 (mastery arena) perf, evidence and recording; the mastery wave GIF; the owner packet, scorecards, CA2 metrics and the known-gaps list (PT-GATE-B).
- The dynamite journal line in a live recording.
- Tactical and spectacular route GIFs per setpiece, and the one-GIF-per-ability list: not part of this stage.
