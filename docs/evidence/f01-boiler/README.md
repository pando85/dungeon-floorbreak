# BOIL-F evidence: the Goblin Boiler at speed, solo and two players

Real windowed captures (`floor01_scene`, the Bevy presentation, 640 x 360 virtual frame, plus one 640 x 400 clip) of `--scene f01_late --f01-room boiler` with its live roster (three goblins and the Engineer as B3, the BOIL-E staging) under the story loadout (`--carl-prog --donut-prog`). The input is a **room pilot** (`src/room_pilots.rs`), a deterministic function of the simulation state, so every clip is reproducible and its hash is pinned. The art on screen is the BOIL-E code-drawn prop stand-ins and prototype sprites (`prototype_fixture_unapproved`); nothing here approves art, tuning or game feel.

This folder is also the first use of the **room capture path** (`scripts/room_capture.sh`, `scripts/room_gif.py`, `--room-pilot`; documented in `docs/03-tech/floor01-scene.md`, "Room capture path"). HAZ-C and the other room stages reuse it.

## Regenerate

```bash
cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh
bash scripts/make_f01_boiler_evidence.sh      # needs xvfb-run and Pillow; ~3 minutes
python3 scripts/check_docs.py                 # verifies SHA256SUMS
```

## Clips (GIFs play at game speed: 6 ticks = 100 ms per frame, 3 ticks for the B3 clip)

| File | Plan and window | What happens (ticks of the run) |
| --- | --- | --- |
| `boiler-direct-solo.gif`, `boiler-direct-2p.gif` | `direct` / `direct:2p`, ticks 180 to 534 (to 474 for 2P) | The three goblins and the Engineer are fought straight through; no prop is touched. Goblin B2 dies at 203; B3's goblin (behind the drum, awake since Carl crossed x 560 at tick 184) takes two dive stomps (366, 385) and falls at 391; the Engineer falls at 479 (555 in 2P, where he lands one hit on tick 515). Solo Carl takes no damage; exit at 534 (631 in 2P). |
| `boiler-valve-solo.gif`, `boiler-valve-2p.gif` | `valve` / `valve:2p`, ticks 160 to 424 | Two jabs on the valve (184, 214): warning at 214, venting at 237, spent at 309; the steam reaches the brace (237, weakened only). Exit at 640 (654 in 2P). |
| `boiler-chain-solo.gif`, `boiler-chain-2p.gif`, `boiler-chain-solo-640x400.gif` | `chain` / `chain:2p` (and `--deck`), ticks 440 to 794 | The full chain: Carl lures a goblin and kicks it (push kick, 477); the launched body sets the valve off (489, `LaunchedBody`); venting at 512, the steam weakens the brace; spent at 584; a jab at 724 releases the brace, it falls (747), lands on the drum (755, drum warning), the drum blasts at 779 and defeats a goblin credited to Carl (`cause=explosion`). Exit at 918 (918 in 2P). |
| `boiler-b3-wake-chain-solo.gif` | `chain`, ticks 630 to 675, 3 ticks per frame | B3 wakes at the trigger column (tick 645, Carl at x 560): the owner game-feel note below. |

`capture_lines.txt` holds one `ROOM_CAPTURE ... match=yes` line per clip. `boiler-chain-solo-640x400.gif` ends on a different hash from `boiler-chain-solo.gif` (`fbc0b9da...` against `e1f261e9...`) for the same plan and ticks: the view height (360 or 400) is an input of the simulation, so it is a different run, not nondeterminism; each clip matches its own headless replay. The 2P clips are also run once more in the live (timed) window loop and must end on the same hash (`timed_hash` in `capture_lines.txt`). GIFs with a 3 or 6 tick step play at game speed; the CP-E 2 tick clips round to 30 ms and play about 10 % fast. `pilot_hashes.txt` is the final state of every plan (headless), also compared by `bash scripts/ci_floor01_combat.sh headless` (`F01_BOILER_PILOTS_OK`).

## Read-only presentation

Each clip is run twice: in a window under `xvfb-run` and headless to the same last tick. The final `COMBAT_STATE` hash of the two must be equal; `capture_lines.txt` shows `window_hash == headless_hash` for every clip. The pilot reads only the finished simulation state, so drawing cannot change what it does. The unit tests (`room_pilots::tests`) check that each plan clears the live room without a defeat, replays to the same hash, and (chain) runs launch, steam, brace, landing and drum once each and in that order.

## What the frames show (looked at by the author, not an art or feel verdict)

- The room is readable at 640 x 360: the lit brick lane, the exit portcullis on the right, goblins and the Engineer as small figures about 230 px apart.
- The valve warning is a gold outline on the valve box; the steam is a near-opaque cream and pale blue block on the lane (688 to 768), low in contrast against the lit wall; an orange block marks the weight's footprint and its fall; the drum blast is a red and orange fireball with the System notice `LAUNCHED!` in the 2P clip.
- The boiler landmark is a large flat brown rectangle (the code-drawn stand-in): it dominates the middle of the frame and carries no detail. This is the BOIL-E stand-in, not final art.
- In solo, Donut (the follower bot) stays beside Carl. In 2P the HUD shows P2 on Donut's plate (Missile and Pounce) and she follows Carl; the pilot gives her no job in the chain.
- The kick, the flying goblin and the valve's warning are readable in the chain clip; the goblin is a small figure while it flies, so the launch is easy to miss at 1x.

## Owner game-feel note: "B3 wakes in the steam" (addressed by FA-Q20, PR-C; the owner has not replayed it)

A recorded owner game-feel note, from the orchestrator handoff (no other source in the repository; the wording is the handoff's, not a quote of the owner): "B3 wakes in the steam". Before PR-C the pilots showed:

- B3 is the goblin at x 944 and the Engineer at x 1000. They woke by **sight** only (220 px): the sidecar's `trigger_x` (560) and `delay_ticks` (0 and 90) were not read. B3's goblin first moved when Carl reached x about 725 (944 minus 220), a column **inside the steam lane** (x 688 to 768). The pilots never reproduced the note (the steam was spent by then), but the wake and the lane coincided in space.
- **Since PR-C (FA-Q20, amends BOILE-AD2)** the live roster honours `trigger_x` and `delay_ticks`: B3's goblin wakes on the first tick any hero is within sight **or at x 560 or more**, and the Engineer 90 ticks after. That column is 128 px before the lane, so no hero stands in the lane at B3's wake tick (the test `boil_b3_wakes_before_the_lane`, four hero modes at nine seeds). B3's goblin first moves at tick 184 in `direct`, 330 in `valve` and 645 in `chain` (the clip `boiler-b3-wake-chain-solo.gif` now films ticks 630 to 675).
- The drum's solid (848 to 880) stands between B3 and the lane: a woken B3 walks to the drum's face (x 892) and waits there until the drum blows or Carl comes over. The `chain` run blasts the drum at 779 and defeats the goblin with it (`cause=explosion`). A test pins the finding and another vents the valve with B3 in the lane (`boil_b3_vented_in_the_lane`, with the goblin stood in the lane by the test setup).
- The earlier wake costs the direct route a hit: 2P `direct` lands one Engineer hit (tick 515) and the scripted library `direct` runs take 1 to 2 hurts (the scorecard), where they took 1 before. Every run still clears 9 of 9. Game feel is for the owner; a human pad session is **NOT RUN**.
- Knobs: `fa_q20_wake_gates` (`assets/data/gameplay/f01_boiler_fixes_v0.json`, default on; off restores the sight-only wake and the old hashes).

## NOT RUN and limits

- **Solo Donut, checkpoint and respawn, and camera: NOT RUN.** The work order names solo Donut (a swap run), the checkpoint and respawn in the room, replays and the camera. No solo-Donut clip was captured, no pilot dies and respawns in the Boiler, and the camera is only what the clips show. BOIL-F is therefore not complete against the work order.
- **Human pad evaluation: NOT RUN.** Nothing here was played on a pad; the owner's gamepad pass of the Boiler is still pending.
- **Real GPU frame time and gore preset 4 cost: NOT RUN.** The captures use the xvfb software rasterizer; no frame-time number is claimed.
- The pilots are not players: they never miss a prompt and never drop a block, and the chain plan lures one particular goblin. The `valve` and `direct` plans leave the weight and drum untouched or the chain unused by design.
- Companion mode (`cf02_free`) fixtures for the Boiler are not part of this stage.
- The replay viewer (`--replay-session`) still reports `unsupported_source` for `f01_room` sessions; this folder does not use it.
