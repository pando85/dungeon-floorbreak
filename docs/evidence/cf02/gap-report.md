# CF02-00 gap report: the Q23 defeat and camera audit

**Scope and honesty rules.** This is the factual baseline CF02-Q23 asks for before any death art is drawn: for each spectacle the owner's DEMO-01 report did not see (B2 contact, B3 decapitation, B5 blast, B4 llama heavy versus plain finish) and for the camera motion the owner called abrupt, it says what the **deterministic autopilot route** produced and whether the **presentation camera** showed it. Every number comes from a replay of the committed build (`scripts/cf02_baseline_audit.py`, tables in [`audit/`](audit/)); every picture is a real 640x360 frame of the Bevy render loop ([`captures/`](captures/)). Statements marked **Hypothesis** are not findings. The autopilot is not the owner: it shows what the route can produce, not what the owner did. The owner's own run was not recorded (no build, flags, device or log, see the [playtest report](../../04-production/demo-01-playtest-2026-11-20.md)), so no row below proves why a spectacle was missed.

Build under audit: the DEMO-01 behaviour of `main` (`--demo`, hashes unchanged by this PR), camera `auto` (the demo default) and `scroll`, output 1280x720 (slot 640x360) and, for zoom, 1920x1080.

## Summary

| Item | Did the autopilot route produce it? | Was it inside the presentation camera? | What it means for the next PRs |
| --- | --- | --- | --- |
| B2 contact (Q3B, owner 1/5) | Yes: `goblin_contact` on tick 75 (`launch=1 source=2 target=3`) | Yes, both bodies 243 to 267 px inside the view (auto and scroll) | The camera is not the cause on this route. The two bodies are 24 px apart, so one silhouette overlaps the other, and the second goblin only staggers: it stays in `Hurt` for at least 20 ticks (back to `Neutral` by +30) and moves **2.0 px** (x 1255.2 to 1257.2) in the first 30 ticks. **Hypothesis:** a visible but limp response explains the 1/5; CF02-B (Q09 knockdown) addresses exactly this |
| B3 decapitation (Q8A "No") | Yes: `enemy_defeated cause=shore_block_edge` (goblin 5) and a `shore_block` crush (goblin 4) on tick 159 | Yes, 176 and 216 px inside | Visible but small: in the 1x frame the head and the standing neck are separable only in a 4x crop. Eligibility is geometric (the head under the stone, the torso outside it), so it needs a goblin on the footprint edge; a different approach gives a crush instead. Not known whether the owner broke the shore with goblins in place |
| B5 blast (Q8B "No") | Yes: `blast` on tick 82 (point blank defeat of goblin 14, ring defeat of goblin 15) | Yes, 190 and 218 px inside | The charge is a QA prop armed when Carl passes column 2640 and blows 75 ticks later; the owner may never have triggered it (unknown). When it blows the burst is on screen |
| B4 llama finishes (Q9B "No noté diferencia") | Only one: the lethal push kick (`force=heavy`, the slam) on tick 277 of the B4 probe and 1161 of the whole route | Yes (+276 px); the tell on tick 20 is 29 px from the edge under `scroll`, 79 px under `auto` | The hall has **exactly one llama** (`DEMO01_LLAMAS: [LlamaSpawn; 1]`) and the route kills it only with a kick, so a player sees one finish per attempt and cannot compare a slam with a plain drop inside one run. CF02-D must therefore not assume the difference was ever shown |
| Camera pan, kick and zoom | Yes | n/a | No pan snap or cut in any of six beats; zoom never fired (see below); the only hard velocity step is the legacy solo-swap pan |

## B2 contact: event versus response

Event log (`docs/evidence/demo-01/headless_demo01_b2.txt`): tick 73 `hit move=push_kick target=2 ... health_left=1`, tick 75 `goblin_contact launch=1 source=2 target=3`. Positions and views are in [`audit/audit_events.md`](audit/audit_events.md). Frames: [t75](captures/b2_contact_t75_640x360.png), [t85](captures/b2_contact_t85_640x360.png), [t100](captures/b2_contact_t100_640x360.png) and the lethal kick on tick 125 ([t125](captures/b2_lethal_kick_t125_640x360.png), `LAUNCHED!` notice).

* The kicked goblin is a 24 px neighbour of the second one when it lands, so at 1x the two sprites overlap into one shape.
* The second goblin's track after the contact is `Hurt` at +0, +5, +10 and +20 ticks and `Neutral` at +30, at x 1255.2, 1256.2, 1257.2, 1257.2, 1257.2: it never falls.
* The actors occupy a strip about 20 px tall at the bottom of the 640x360 frame; the rest is architecture.

## B3, B5: spectacle on screen but small

* B3 (frames [t159](captures/b3_block_landed_t159_640x360.png), [t165](captures/b3_block_landed_t165_640x360.png)): the block has landed, the notice reads `FLATTENED!`, one goblin lies under rubble and a second stands headless with a blood fountain and the head on the floor. A 4x crop shows it; at 1x it is a few pixels.
* B5 (frames [t82](captures/b5_blast_t82_640x360.png), [t86](captures/b5_blast_t86_640x360.png)): a fireball between two goblins on the blast tick and a larger one four ticks later; a HUD notice reading `FLATTENED!` is up in both frames.

## B4 llama

Frames: the spit tell and glob ([t56](captures/b4_llama_spit_t56_640x360.png)), the slam kick ([t277](captures/b4_llama_slam_kick_t277_640x360.png), [t290](captures/b4_llama_slam_kick_t290_640x360.png)). Events: `llama_tell` tick 20, `llama_spit` tick 56, jabs and kicks on the chaser and the llama, `enemy_defeated target=17 force=heavy` on tick 277. No plain (jab or haymaker) llama kill occurs on the autopilot route, so no capture of that finish exists in this PR; producing one needs a fixture or a manual run (**NOT RUN**).

## Camera

Full tables: [`audit/camera_trace.md`](audit/camera_trace.md).

* **Pan.** The presentation camera moves at most 4 px/tick in five of the six beats. In B4 (`auto`) it eases from 5 px/tick (tick 9) up to 10 (ticks 13 to 16) and back to 5 by tick 25 while it frames the llama; no tick moves the view by more than 10 px and there is no cut (a pan above 40 px) in any beat or camera.
* **Impact kick.** At most 3 px, on 7 to 35 ticks per beat.
* **Zoom.** The view width stays 640 on every tick of every beat at both outputs (1280x720 and 1920x1080 at scale 3). The zoom out when the llama takes aim was **not reproduced** by this trace; the cause was not investigated here. This agrees with the owner's "not observed" (Q10A).
* **Solo swap pan (legacy).** The `solo_swap_continuity` fixture keeps the heroes 40 to 70 px apart, so its swap moves the camera by 0 px. With a 309 px gap (the setup of the party test `q7_a_swap_pan_...`) the scene camera pans at the **constant 8 px/tick cap from the first tick**, 18 identical steps, then 5 and a dead stop: a velocity step of 8 px/tick at the start and 5 px/tick at the end. The new test `cf02_baseline_swap_pan_trace` pins this baseline so CF02-C2's eased pan (Q20) is a deliberate change. **Hypothesis:** this hard start and stop is the "movimientos bruscos" the owner reported; only an owner capture of the moment can settle it (design section 8, question 4).

## What could not be produced

* **The owner's route and the owner's view.** No log or recording exists. Everything above is the autopilot route.
* **A plain-kill llama finish and a human camera trace.** Not recorded by CF02-00; listed as NOT RUN for the next playtest ([scorecard](scorecard-template.md)).
* **Why the zoom did not fire.** Recorded as an observation, not diagnosed.
* **Frame time on the target hardware**, real two-pad 2P, the checkpoint respawn and the new sounds: NOT RUN.
