# DR-A to DR-D evidence: room 3, the Dozer revenge ("The Machine Remembers")

Real windowed Bevy captures (`floor01_scene`, 640 x 360 and 640 x 400 virtual frames) of `--scene f01_late --f01-room dozer` under the story loadout: Carl under the pilot, Donut the autonomous `cf02_free` AI partner (`COMPANION_CONFIG: set=cf02_free`, the mode a player of the room sees). The machine is drawn with the approved **DOZ-01** rig (hull, tread, riders, front cab and tub, spiked wheel, yoke, smoke and steam puffs; [`../doz-01`](../doz-01/README.md)); there is **no box stand-in** for it. The rear gate, the buffer stop and the damage marker are code-drawn **labelled stand-ins** with a `STAND-IN` tag until DOZ-02 draws them (DZ-AD13); since DR-B so are the stop frame, its chain, its floor shadow, the brace (intact, weakened, snapped), the remnant and the rear glance. Nothing here approves tuning, art or game feel.

The input is the room pilot `dozer:close_call` (`src/room_pilots.rs`, `dozer_escape`): Carl walks to x 480, holds there until the wheel is 64 px away, then runs to the exit. Carl's input is a function of the simulation state only (DRA-AD2: stateless). The pilot is **aligned** (DRA-AD5, the #282 review F1): the party is the `cf02_free` partner's, so Donut is driven by the AI partner, not by the legacy follower; the room takes no `--cf02-seed` (the aligned seed 0 is nominal), like a player's run. Every capture is reproducible.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace && bash scripts/export_assets.sh
bash scripts/make_dozer_revenge_evidence.sh   # needs xvfb-run, ImageMagick and ffmpeg; ~25 minutes
python3 scripts/check_docs.py                 # verifies SHA256SUMS
```

`perf.txt` is made on its own with `scripts/room_perf.sh dozer dozer:close_call 499` (three runs per view, plus `--deck` for 640 x 400; since DR-D also the three-build A/B of `dozer:close_call`, `dozer:brace`, `dozer:crew` and `dozer:retry`, each build run from its own worktree with `BIN` and `OUT`) and the `F01_DOZER_NAV` line of the headless room smoke (`--f01-room dozer --room-pilot <plan> --exit-after-ticks 520`).

## Stills (tick of the `dozer:close_call` run)

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `trigger_*.png` | 30 | The rumble: Carl has crossed the trigger, the rear gate stand-in shakes and smoke rises over it; the machine is not drawn yet (it is still behind the gate). |
| `chase_*.png` | 200 | The committed chase (cruise band): the machine rolls right, wheel and tread turning by distance, the riders on the hull; Carl holds at x 480 with Donut beside him. |
| `close_call_*.png` | 242 | The mercy band and the close call: the wheel is a few px from Carl, the riders yell (within 120 px), Carl starts to run. |
| `run_*.png` | 306 | Cruise again: Carl and the AI partner Donut run ahead of the machine. |
| `brake_*.png` | 476 | The brake at the buffer stop stand-in: the riders lurch forward, the machine slides 4 px. |
| `exit_*.png` | 498 | Carl reaches the exit under the lintel; the machine strains at the buffer stop (the engineer holds the lever, the riders jeer). |

Each still exists at `640x360` (`--scale 1`) and `640x400` (`--deck --scale 1`). `chase_640x360.gif` is ticks 174 to 498, every 6th tick (55 frames, 10 frames a second, 0.6x game speed).

Timeline of the run (the headless `COMBAT_EVENT` lines): rumble 21, emerged 111, bands catch_up, cruise from 132, mercy from 237, cruise from 297, brake 473, escaped 498. Without a pilot (the smoke's walk right) the exit is reached on tick 344. Every `DOZER_STILL` and `DOZER_GIF` line carries `partner=cf02_free`, read from the capture's own `COMPANION_CONFIG` line; the script fails on any other value.

## DR-B: the reversal (`dozer:brace`)

The room pilot `dozer:brace` (`src/room_pilots.rs`, `dozer_brace`) is the live twin of the `dz_frame_disable_once` fixture: Carl walks to x 856, turns back to the brace at x 832 (so he faces it), and on the machine's first committed-chase tick aims the release at the middle of the helper's useful window (`dozer::release_window` for a hero at the brace, read from the live frame: R 163 to 246, so R 204). He jabs the brace twice, holds while the frame lands on the wheel front, and runs once the machine is stunned. Donut is the autonomous `cf02_free` partner, as in the escape plans. The pilot keeps the first chase tick, so it is aligned only.

Timeline (the headless `COMBAT_EVENT` lines): emerged 111, the jabs hit the brace on 303 (weakened) and 315 (released, R 204, credited to player 0), the frame falls from 344 and lands on 352 on the front collider with the wheel front at x 728 (`impact_stunned`, `by=0`), the machine is `disabled` on 400 (the 48-tick stun) and the crew dismounts, and the room completes on 490. Nobody is hurt and the wheel hits nobody (the unit test `the_dozer_brace_pilot_reverses_the_chase`).

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `brace_release_*.png` | 318 | The warning: the brace stand-in snapped, the frame shakes on its chain, its shadow on the floor; Carl at the brace, the machine coming. |
| `brace_fall_*.png` | 348 | The frame falls (no chain) onto the footprint; the wheel front is under it. |
| `brace_stun_*.png` | 353 | The landing on the front: the machine is stunned (the damage marker stand-in), the remnant lies under the jammed front. |
| `brace_disabled_*.png` | 404 | The machine is disabled; Carl runs for the exit. |
| `brace_run_*.png` | 440 | Carl and Donut run past the dead machine to the exit. |

## DR-C: the crew (`dozer:brace`)

The two roster goblins of room 3 (`rider_a`, brawler, and `rider_b`, coward) ride the machine: while carried they are out of play, hidden by the presenter and drawn only as the DOZ-01 rig's seated riders. They dismount on the disable tick (400) at the wheel front plus their landing offset (x 777.9 and 817.9 in this run; the front is at 745.9), facing the nearest hero and down for 45 ticks. From the disable on, the rig draws no seated rider, so a goblin is never drawn twice. `brace_disabled_*.png` (404) shows both riders down beside the wheel; `brace_run_*.png` (440) shows them up while Carl and Donut run for the exit. The crew rules (bounded living count, the restart rules, the retries from a stunned or disabled machine) are fixtures (`dz_crew_bounded`, `dz_respawn_after_disable`, `dz_retry_stunned`, `dz_retry_disabled`, the `dz_crew_companion` pilots), not captures.

`reversal_640x360.gif` is ticks 297 to 489, every 4th tick (49 frames, 10 frames a second, 0.4x game speed): the second jab, the warning, the fall, the stun, the disable and the run.

## DR-D: effects, retry and crew

**Effects (INT DOZER-ROOM, audit-art work order 15).** The presenter now draws the DOZ-01 metal hit-spark (`vfx_hit_spark_metal`) on the hull face for each non-blast clang (a melee hit or a Magic Missile) and the ENV-02 splinter burst (`floor01_splinter_burst`) on the brace cleat for each counted brace hit and the release, and twice in the rear gate as the machine emerges (DRD-AD1, DRD-AD2; [floor01-scene.md](../../03-tech/floor01-scene.md#room-3s-effects-retry-and-crew-captures-dr-d)). They draw in front of the actors. Since DR-D the DR-B stills show them too: `brace_release_*.png` (318) has the release burst (released on 315) on the brace. The burst is sparse wood flecks of the ENV-02 palette and reads weakly over the brick at 1x (a note for the owner and DOZ-02, not a defect of this wiring).

**Retry (`dozer:retry`, the #289 review finding 4).** Carl stands at x 480 until the wheel takes him (one blow, DZ-Q04), respawns at the checkpoint, walks into the trigger, stands while the gate rumbles and runs once the machine is out. Timeline: defeat 273 (`wheel_hit`, `hero_defeated`), checkpoint respawn 322 (49 ticks, 0.82 s, after the defeat; the world tick runs on), rumble 343 (by the trigger), out of the gate 433 (the gate bursts on 433 to 452), exit 756. Defeat to the next chase is 160 ticks (2.7 s); respawn to the next chase is 111 ticks (1.85 s), of which 90 are the rumble. The HUD title still reads `TRY 1` after a respawn: the try count is of defeat retries (restarts), and a checkpoint respawn is not one.

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `retry_caught_*.png` | 270 | The wheel closes on Carl, who holds at x 480. |
| `retry_down_*.png` | 300 | Carl is down, his health empty; the machine rolls on. |
| `retry_respawn_*.png` | 330 | Carl back at the checkpoint by the gate stand-in, full health; the machine is gone. |
| `retry_gate_*.png` | 438 | The machine leaves the rear gate again; the gate's sparse splinter flecks over the wheel; Carl runs. |
| `retry_escape_*.png` | 520 | Carl and Donut run ahead of the machine. |

`retry_640x360.gif` is ticks 246 to 780, every 6th tick (90 frames, 10 frames a second, 0.6x game speed).

**Rider fight (`dozer:crew`, finding 4).** The `dozer:brace` reversal, then the CF02 fight of the `direct` plan against the two dismounted riders, then the exit. Timeline: released 315, stunned 352, disabled 400 (both riders dismount), Carl's haymaker puts rider B down on 463, Donut's first missile hits rider A on 488, Carl's jab puts rider A down on 515 and also clangs on the disabled hull (the hit-spark, 515 to 520), Donut's second missile clangs on the hull on 523 (a second spark), exit 693. Nobody is hurt or run over (`the_dozer_retry_and_crew_pilots_finish_room_3`).

| Name | Tick | What the frame shows |
| --- | --- | --- |
| `crew_dismount_*.png` | 410 | The machine disabled, both riders down (crouched) beside the wheel, Donut next to them; Carl comes back from the right. |
| `crew_fight_*.png` | 452 | Carl fights the two riders in front of the wheel; Donut behind him. |
| `crew_clang_*.png` | 516 | Rider B lies on the floor, rider A goes down; the small metal hit-spark of the jab's clang at Carl's fist, in front of the wheel; a Donut missile flies in from the right. |
| `crew_exit_*.png` | 600 | Both riders down on the floor; Carl and Donut walk away from the dead machine to the exit. |

`crew_640x360.gif` is ticks 396 to 692, every 4th tick (75 frames, 10 frames a second, 0.4x game speed).

## Read-only presentation

- `capture_lines.txt`: one `DOZER_STILL` line per still and size, one `DOZER_GIF` line per GIF. Every capture ran in the window under `xvfb-run` and headless to the same last tick: `window_hash == headless_hash` (`hash_match=yes`) for all 40 stills (20 names at two sizes) and the four GIFs.
- Every still was captured in two full passes; the PNGs of the two passes are byte-identical (`passes_byte_identical=yes`).
- The unit test `dozer_art::tests::drawing_never_changes_the_state_hash` runs the room with and without calling the presenter every tick and compares the state hash on every tick; `the_frame_stand_ins_follow_the_frame_and_the_rear_glance` (DR-B) does the same through a release, a landing and a rear glance, and checks every frame, brace, shadow, remnant and glance label.

## PLAYTRACE session

`playtrace_session.txt`: one session of the DR-B pilot `dozer:brace` recorded in the real window (`--record --exit-after-ticks 520`; the frame lands on 352, the machine is disabled on 400, the room completes on 490), verified with the late-Floor-1 replay rebuild: `PLAYTRACE_SAVED ticks=520 status=Completed`, `PLAYTRACE_VERIFY result=PASS scope=full ticks=520 episodes=1 combat_events=14 control_events=5 build=identical`, `state_hashes=520 event_hashes=520`. `effective_inputs=520` and the `COMPANION_CONFIG: set=cf02_free` line: the aligned pilot's inputs are recorded and replayed through the partner's party (the replay rebuilds `live_tuning` from the manifest flag `companion=cf02_free`). Session ids and build ids are stripped. (DR-A recorded `dozer:close_call` here, 8 combat events.)

## Frame time and nav rebuilds (`perf.txt`)

Debug build, xvfb (not a real display), AMD Ryzen 9 5950X, NVIDIA GeForce RTX 4060 Ti, 499 frames a run (the exit tick 498 plus one), 120 warm-up frames, the `cf02_free` partner active in every run (a `# COMPANION_CONFIG: set=cf02_free` line follows each run in `perf.txt`), load average 4.0 to 9.5 after the runs (`loadavg_after` in `perf.txt`; the machine was shared with other builds):

| View | Median frame (3 runs) | p95 frame | Sim tick mean / p95 | GPU mean | Entities |
| --- | --- | --- | --- | --- | --- |
| 640 x 360 | 16.22 to 16.30 ms | 19.81 to 20.74 ms | 66.6 to 69.1 us / 314.7 to 315.0 us | 1.82 to 1.84 ms | 5544 |
| 640 x 400 | 16.35 to 16.36 ms | 19.91 to 20.07 ms | 64.9 to 66.4 us / 304.3 to 308.4 us | 1.87 to 1.91 ms | 5544 |

The frame interval is the xvfb present pace, not a real-display frame time. The simulation tick's p95 is the cost of the partner's nav rebuilds, not of per-tick planning: 62 of the 499 ticks of a run (12 %) rebuild the partner's nav graph from the wheel's hazard rows, so the 95th percentile falls on a rebuild tick, while the mean is the ordinary tick. The #282 re-review read about 63 us at p95 for the partner-active mastery arena, which has no moving hazard rows. **This PR's cost (the #287 review F2).** The first head of DR-B kept the stop frame's remnant in the partner's nav from tick 1, which made every rebuild larger: the reviewer's A/B (main against that head, same frame counts, low load) read about +30 % on the tick mean and +40 % on the p95. With the remnant out of the nav until it lands, a paired A/B (main and this head alternating, `dozer:close_call` at 640 x 360, five pairs at load average 8 to 10; the raw per-run numbers are the `DR-B A/B` table at the end of `perf.txt`) reads a median paired difference of +6 us on the tick mean (about +9 %) and +5 us on the p95 (about +2 %); the frame median does not change. The residual is probably the per-tick step of the frame's prop and hazard; it is not profiled (follow-up DOZER-PERF-01), and a debug build under xvfb is no benchmark. The earlier measurement of DR-A, about 17 us, ran the follower by mistake (the #282 review F1).

**Nav rebuild count, with the partner active.** The `cf02_free` partner's hazard feed (the wheel's nav rows) changed **63** times in `dozer:run` (344 ticks), **62** in `dozer:close_call` (498 ticks), **62** in `dozer:wait` (498 ticks) and **46** in `dozer:brace` (490 ticks: the machine stops on 352), with `hazard_feed_errors=0`, `wheel_hits=0` and no rescue (`F01_DOZER_NAV` lines in `perf.txt`). The machine is a hazard the partner's nav reads; the rows are filled into a reused buffer (`Dozer::nav_rows_into`), so a tick allocates nothing for them. The no-pilot smoke reads 0 (`COMPANION_CONFIG: set=cf02_free inactive`): its scripted walk runs the legacy follower, not the partner, and is a CI smoke, not the mode a player sees. The unit test `the_dozer_pilots_escape_room_3` asserts the count of every dozer pilot is above 0 (the #282 review R2). The earlier claim of this README (0 rebuilds, "CA2-FU-HAZ U-1 stays open") was measured with the follower by mistake and was wrong.

**DR-C and DR-D A/B (the #289 review finding 3, DOZER-PERF-01).** Three debug builds ran in turn on the same machine, five reps each of `dozer:close_call` (499 frames) and `dozer:brace` (491 frames) at 640 x 360: `predrc` (main before DR-C, f2087869), `main` (the DR-C merge, 49b1b0fe) and `drd` (this head). Reps 1 to 4 ran at load average 1.6 to 3.3, rep 5 at 6.9 to 10.9; the raw per-run numbers are the `DR-C/DR-D A/B` table at the end of `perf.txt`. Median paired differences:

| Plan | Step | Sim tick mean | Sim tick p95 | Frame median |
| --- | --- | --- | --- | --- |
| `dozer:close_call` | DR-C (main against predrc) | -1.4 us (-2.0 %) | -1.0 us (-0.3 %) | -0.05 ms |
| `dozer:close_call` | DR-D (drd against main) | +3.3 us (+4.8 %) | +10.9 us (+3.5 %) | +0.02 ms |
| `dozer:brace` | DR-C (main against predrc) | +2.1 us (+3.4 %) | +10.6 us (+3.5 %) | +0.05 ms |
| `dozer:brace` | DR-D (drd against main) | +2.2 us (+3.5 %) | -1.5 us (-0.5 %) | -0.03 ms |

DR-C costs nothing in the escape (the riders are carried and out of play) and a few us in the brace run, where the two dismounted goblins walk and plan from tick 400. DR-D reads a few us more on the tick mean in both, but it changes no simulation code on these two plans (the effects are presentation outside the tick, the two new pilots are not run here, and the four pooled effect sprites, 5561 to 5565 entities, are drawn outside it), so that residual is not this PR's tick work: it is within the run-to-run spread of a rep (the brace p95 goes the other way) and probably build layout or noise. No step moves the frame median. A debug build under xvfb is no benchmark, and nothing here is profiled; DOZER-PERF-01 stays a measure-again item for a release build on a real display. The new `dozer:crew` and `dozer:retry` plans have no main counterpart; their runs (two per plan at 640 x 360 and one `--deck`, at load average 5.8 to 10.9) read a frame median of 16.3 to 18.6 ms and a tick mean of 61 to 113 us, in line with the other plans at that load. Their nav lines (the headless smoke stops at 600 ticks) read 46 rebuilds for `dozer:crew` (the machine stops on 352, as in `dozer:brace`) and 64 for `dozer:retry`, both with `hazard_feed_errors=0`; `close_call` (62) and `brace` (46) are unchanged.

## NOT RUN and limits

- **Human pad evaluation: NOT RUN.** Game feel is the owner's; nothing here was played on a pad.
- **Real-display frame time and release build: NOT RUN.** The numbers above are debug under xvfb.
- **Two players, solo Donut and a two-hero wipe: NOT RUN.** The two-pad and Donut-missile releases are fixtures (`dz_frame_missile`, `dz_frame_dynamite`), not captures; the retry captured here is Carl's own defeat and checkpoint respawn with the partner alive, not a party wipe and restart (that path is the fixtures `dz_retry_stunned`, `dz_retry_disabled`).
- **The DOZ-02 art (gate, buffer stop, damage states): NOT RUN here.** Its key design is draft PR #291; the stand-ins stay until its execution lands.
- The pilots are not players: they never misjudge the gap.
- **Stand-in limits (the #287 review notes).** The stand-in pools' caps (`MAX_STAND_INS` 24, `MAX_TAGS` 6 in `dozer_art.rs`) are checked by `debug_assert!` only, so a release build would not catch an overflow; the current worst case fits. The grey frame and remnant stand-ins (FrameIron) read weakly against the dark wheel at 1x; left to DOZ-02 and the owner.

## Rights and provenance

- DOZ-01 rig: designed by Claude Opus, size decided by a separate Claude Opus agent, executed by Claude Sonnet, reviewed by independent Claude Opus agents and approved by the owner ([`../doz-01`](../doz-01/README.md)); the riders reuse GOB-01 goblin pixels.
- DR-A wiring, the labelled stand-ins and these captures: Claude Opus, as assigned by the orchestrator's brief. DR-B (the frame, the brace and glance stand-ins, the `dozer:brace` pilot and the reversal captures): Claude Opus, as assigned by the orchestrator's brief; the stand-ins are code-drawn rectangles, no new art. DR-D (the effect wiring, the `dozer:retry` and `dozer:crew` pilots, these captures and the perf A/B): Claude Opus, as assigned by the orchestrator's brief; no new art: the hit-spark is the approved DOZ-01 `vfx_hit_spark_metal` sheet and the burst is the approved ENV-02 `floor01_splinter_burst` sheet.
- Original fan-concept art published under the owner-reported `ART-PUBLICATION-ALL-V1` scope (`docs/04-production/rights-and-release.md`). No third-party assets.
