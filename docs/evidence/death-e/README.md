# DEATH-E1 evidence: intensity-4 goblin validation and first measured budgets

**Status:** measured and captured, **nobody has judged the look or the feel yet** (owner game-feel review and independent review pending). Everything here was measured on a **development machine, not on the Steam Deck**: no Steam Deck figure, no Deck-verified claim and no lower-spec measurement exists. Design and behaviour: [DEATH-E1 in the floor 01 scene doc](../../03-tech/floor01-scene.md#death-e1-goblin-validation-and-budgets).

## What was measured, and on what

| | |
| --- | --- |
| CPU | AMD Ryzen 9 5950X (16 cores, 32 threads), 64 GB RAM |
| GPU | NVIDIA GeForce RTX 4060 Ti 16 GB, Vulkan (driver 615.71), drawing into an Xvfb virtual display |
| Build | `cargo build --locked --release` (the default release profile, rustc 1.100, Bevy 0.20.1), `floor01_scene` |
| Display | Xvfb, 640x360 (`--scale 1`) and 1920x1080 (`--scale 3`), lighting on, `AutoNoVsync` |
| Load | **a heavily shared machine**: other builds and a browser ran throughout (load average 8 to 21 during the final run, see [perf_raw.txt](perf_raw.txt)) |

Scenario: the new fixture `cp03_goblin_pileup` (a presenter stress scenario, not a gameplay design). Ten stationary goblins (five of them goblin variant B) are defeated by three causes inside 65 ticks: three jab defeats on tick 83, four shore-block crushes on tick 139 and three lethal kicks on tick 148. At preset 4 that draws, without any pool limit, a peak of **128 launched pieces, 130 floor decals and 18 wall spatters**. `--death-stress` additionally keeps every pool completely full (synthetic crushes and decapitations every third tick, recycled thousands of times), which is the max-load row. By tick 268 all ten corpses have settled, so the two oldest retire (the corpse limit below) inside the probe window.

The probe (`--perf-from-tick 70 --perf-frames 600`, driven by `scripts/measure_death_e1.py`) steps exactly one fixture tick per measured frame after a 120-frame warm-up, so every run covers the same deaths whatever the frame time. Three runs per row; the table gives the median of the runs (maximum of the maxima).

### What the probe does and does not show

* **Frame interval:** under Xvfb it is pinned by the virtual display (16.67 ms at 640x360 even with the present mode set to no-vsync; about 60 to 75 ms at 1920x1080 in every row, including the scene with nobody dying, because the present path copies the whole frame through the X server on the CPU). It only shows whether frames were missed; it carries no cost of the deaths.
* **GPU time: not established.** The probe sums the GPU pass timings (Vulkan timestamp queries) that Bevy records. A first version read only the last camera's value per pass name, which the independent review found did not respond to the sprites at all, so every earlier GPU figure and claim (about 0.1, under 0.15 and under 0.25 ms) is **withdrawn**. The probe now sums every measurement of the newest frame per pass, but on this machine the result cannot be used: the same baseline scene read 0.18 ms at 640x360 and 2.5 ms at 1920x1080, the 1920x1080 baseline was slower than the pileup rows, and the stress rows were faster than the plain pileup at 640x360, so the differences are dominated by contention (the GPU and the CPU were shared) and not by the deaths. The raw values are in [perf_raw.txt](perf_raw.txt). **The GPU cost of the deaths was not measured**; it needs a quiet machine, ideally a Steam Deck-class one. The custom lighting pass has no GPU timer either.
* **CPU, memory, entities and pool counts:** the in-process CPU timers (the fixture tick, with the death presenter inside it, and the sprite sync), resident memory, entity counts and pool peaks are reported below. With the load above they are upper-end, noisy figures: for example RSS moves by tens of MiB between rows of the same resolution with no relation to the load of the scene, so no memory cost of the deaths can be claimed beyond "within the noise".

### Results at the budgets of this PR (parts 256, floor decals 192, wall decals 32)

Source: [perf_table.md](perf_table.md) and the raw lines in [perf_raw.txt](perf_raw.txt).

| Resolution | Scenario | frame ms mean / p95 / max | tick us mean / max | sprite sync us mean / max | RSS MiB (peak) | entities | peak parts / floor / wall / visible sprites | recycled (parts, floor, wall) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 640x360 | baseline, nobody dies | 16.66 / 18.70 / 33.77 | 9.6 / 58 | 8.8 / 67 | 420 (421) | 4092 | 0 / 0 / 0 / 0 | 0, 0, 0 |
| 640x360 | pileup, preset 1 | 16.67 / 19.32 / 44.62 | 14.0 / 107 | 12.0 / 78 | 383 (388) | 4110 | 4 / 0 / 0 / 8 | 0, 0, 0 |
| 640x360 | pileup, preset 4 | 16.69 / 19.80 / 55.22 | 21.9 / 2162 | 20.3 / 768 | 385 (388) | 4110 | 128 / 130 / 18 / 238 | 0, 0, 0 |
| 640x360 | pileup + pools full, preset 1 | 16.98 / 21.77 / 61.03 | 36.7 / 540 | 20.6 / 483 | 386 (388) | 4110 | 227 / 0 / 0 / 231 | 0, 0, 0 |
| 640x360 | pileup + pools full, preset 4 | 16.93 / 20.97 / 82.84 | 95.4 / 2555 | 38.1 / 209 | 387 (391) | 4110 | 256 / 192 / 32 / 730 | 1814, 2661, 442 |
| 1920x1080 | baseline, nobody dies | 74.35 / 103.57 / 353.47 | 15.5 / 78 | 11.4 / 68 | 394 (396) | 4092 | 0 / 0 / 0 / 0 | 0, 0, 0 |
| 1920x1080 | pileup, preset 1 | 63.93 / 77.20 / 150.94 | 19.2 / 159 | 14.0 / 77 | 398 (435) | 4110 | 4 / 0 / 0 / 8 | 0, 0, 0 |
| 1920x1080 | pileup, preset 4 | 75.76 / 93.56 / 201.81 | 24.0 / 152 | 24.2 / 386 | 401 (435) | 4110 | 128 / 130 / 18 / 238 | 0, 0, 0 |
| 1920x1080 | pileup + pools full, preset 1 | 68.43 / 88.59 / 152.40 | 30.1 / 165 | 18.2 / 131 | 399 (400) | 4110 | 227 / 0 / 0 / 231 | 0, 0, 0 |
| 1920x1080 | pileup + pools full, preset 4 | 68.56 / 95.18 / 170.70 | 87.3 / 1812 | 32.7 / 213 | 397 (398) | 4110 | 256 / 192 / 32 / 730 | 1814, 2661, 442 |

Entities: 4092 with nobody dying and 4110 in the other rows (the death sprites are pre-spawned hidden and reused, nothing is spawned at runtime). "Pools full" rows recycle thousands of pieces by design (each fades out in a spare slot): that is the stress, not a loss.

Reading: the CPU cost of the presentation (the tick includes the whole simulation) stays small even with every pool full at preset 4: about 90 microseconds per tick on average on this loaded machine (about 0.5 percent of a 16.7 ms frame), with isolated spikes of 1 to 2.5 ms that also occur in rows with nobody dying or at preset 1 and are not attributed to the deaths. The mean at full load is higher than the first measurement of this PR (about 40 microseconds, on a quieter machine, before the fade and corpse bookkeeping and before the load grew); this run does not separate the two causes. No frame was missed beyond the noise of the shared machine, but with the frame interval pinned by the display that says little. Everything here must be repeated on a quiet machine and on a Steam Deck-class one.

### Before the budgets (160 / 96 / 12, oldest-first recycling)

The same probe on the build before the budget change ([perf_before_table.md](perf_before_table.md), [perf_before_raw.txt](perf_before_raw.txt); the three melee groups of the fixture stood a few pixels differently, the same ten defeats; its GPU column is the uncorrected last-camera value and must be ignored). The CPU cost was equally small (tick 10 to 32 us, sync 27 to 36 us), but the pileup at preset 4 **ran out of pool**: it recycled 34 floor decals and 6 wall spatters before their time to live (peaks 96 of 96 and 12 of 12). Decals of the first deaths were being taken back while still fresh and, being oldest-first, a corpse's blood pool could go before the splats around it. That is what the new budgets fix, not a cost problem.

## The budgets

All are fields of `DeathBudget` in `src/death_present.rs` (`DEATH_BUDGET`), tested in `death_present::tests`.

| Budget | Before | Now | Why |
| --- | --- | --- | --- |
| launched parts, gibs and drops | 160 | **256** | the pileup needs 128 at preset 4 (2.0 x margin) |
| floor decals (a two-cell pool is one entry) | 96 | **192** | the pileup needs 130 (1.5 x); the old 96 recycled 34 early |
| wall spatters | 12 | **32** | the pileup needs 18 (1.8 x); the old 12 recycled 6 early |
| fade slots (parts / floor / wall) | none | **64 / 48 / 8** | spare sprite slots that hold the entries fading out after a recycle; a burst larger than this takes the oldest fading entry back at once |
| recycle fade | pop | **30 ticks** | a recycled stain or part fades out instead of popping |
| death scripts alive at once | 24 | **48** | real defeats keep their script and pose until the room restarts or the corpse retires |
| settled corpses visible | unlimited | **8** | owner decision 2026-11-19, below |
| corpse fade | n/a | **30 ticks** | older corpses fade out, oldest first, then retire |
| time to live of short pieces (drops, gibs, ear, rags) | 120 ticks, fade 12 | unchanged | motion is the noise |
| time to live of long pieces and every stain | 360 ticks, fade 30 | **none** | orchestrator design decision, below |
| drop splats and part stains per death | 8 and 6 | unchanged | spec section 8 |

**Intensity is not lowered anywhere**: preset 4 draws exactly what it drew (the tests pin it; the budget only grows). A full pool recycles by time-to-live class first (short pieces such as blood drops and gibs, then long ones such as splats and limbs, then kept ones such as the head, the torso, the club, blood pools, smears and wall spatters), oldest first within a class. A fresh drop therefore never evicts a corpse's pool **while any shorter-lived entry remains** (once a pool holds only kept entries, the oldest of them goes), and the pileup recycles nothing at all.

**Corpse limit (owner decision 2026-11-19: "having a limit will help to avoid making things too noisy and confused").** At most 8 settled corpses stay visible. Beyond that the oldest fade out over about 30 ticks, oldest first, and retire: their death script and the parts they launched are freed (their stains stay). It is presentation only: the goblin stays in the simulation and no event or hash changes. The last two frames of the capture sheet show the two oldest corpses fading and gone.

**Time to live (orchestrator design decision 2026-11-19 under antihero-tone, owner game-feel review pending).** The hero walks off through the mess he made, and the mess is the joke. Airborne gibs and drops keep their short time to live (motion is the noise); floor and wall stains, limbs and every other settled long or kept part have no time-based fade and leave only when a pool recycles them, fading out over about 30 ticks instead of popping. The budgets are first measured values from one development machine. They are **not** a Steam Deck figure and must be re-measured on a Deck-class machine; the probe commands below do that unchanged.

## Variant B

Goblins with an odd index (the same parity as the GOB-01 atlases) are variant B: their death poses and parts come from the GOB-02B-DEATH sheets (the same frames, pivots, anchors and part ids plus the `accessory` layer: bandana, ear notch, vest patch), and every piece launched by their death is drawn from the B parts sheet. The accessory is not gore, so it is drawn at preset 1 too (art ruling 13.1a). The pileup has five B goblins (indices 1, 3, 5, 7, 9).

## Captures: Carl and Donut next to the debris

Real `floor01_scene` captures (xvfb, the real render loop, lighting on, one frame per tick), preset 4, ticks 70 to 329 of `cp03_goblin_pileup`; the final `COMBAT_STATE` of each run equals the headless replay (the script fails otherwise).

| File | Content |
| --- | --- |
| [sheet_pileup_p4_640x360.png](sheet_pileup_p4_640x360.png) | the six moments at the native 640x360 |
| [sheet_pileup_p4_actors_1920x1080.png](sheet_pileup_p4_actors_1920x1080.png) | crops of the same six moments at 1920x1080 around Carl, Donut and the gore |
| `still_pileup_impact_1920x1080.png`, `still_pileup_crush_1920x1080.png`, `still_pileup_peak_1920x1080.png`, `still_pileup_settle_1920x1080.png`, `still_pileup_retiring_1920x1080.png`, `still_pileup_retired_1920x1080.png` | the full 1920x1080 frames: impact (tick 85, the jab defeats), crush (141, the block lands), peak gore (158, kick defeats on top of the crush), settle (265, ten settled corpses), retiring (283, the two oldest fade out) and retired (325, gone, stains stay) |
| `pileup_p4_640x360_60fps.mp4`, `pileup_p4_1920x1080_60fps.mp4` | every tick, 60 fps = real time |

![The six moments at 640x360](sheet_pileup_p4_640x360.png)

![The six moments at 1920x1080 around the actors](sheet_pileup_p4_actors_1920x1080.png)

Silhouette check, from the captures and the draw-order tests: **Carl stays readable in all six moments at both resolutions** (he is drawn above every piece of debris, floor decal and wall spatter: debris sits 0.4 and decals 0.5 under the actor layer, unit-tested against every actor offset of the scene) and his outline is never lost against the red and the green. At the 640x360 size the ten-goblin pile reads as a pile, not as individual goblins, which is expected at 1x. **Donut is partly hidden by goblin bodies, not by debris.** At impact (tick 85) and crush (141) the three living goblins of the left group stand on her spot (x 370 to 372, the fixture's placement) and are drawn in front of her, for about 65 ticks until the kick defeats them on tick 148; at the peak frame (158) those three, launched by the kick, still cover her for a few ticks; at settle (265) and later she is clear. This is the existing actor order (a living or airborne goblin is drawn over her), not the presenter. The debris and decals themselves never cover her. A note for the owner, see the design questions in the PR. In the last two frames the two oldest corpses fade out and are gone while their stains stay.

## Reproduce

```text
bash scripts/export_assets.sh
cargo build --locked --release --bin floor01_scene
python3 scripts/measure_death_e1.py        # the probe matrix: perf_raw.txt and perf_table.md (about 25 minutes)
python3 scripts/make_death_e1_evidence.py  # the captures and the sheets (needs xvfb-run, ffmpeg, Pillow)
```

One probe by hand (needs `xvfb-run`):

```text
xvfb-run -a -s "-screen 0 1920x1080x24" ./target/release/floor01_scene --scale 3 --fixture cp03_goblin_pileup --gore-preset 4 --perf-from-tick 70 --perf-frames 600
xvfb-run -a -s "-screen 0 1920x1080x24" ./target/release/floor01_scene --scale 3 --fixture cp03_goblin_pileup --gore-preset 4 --perf-from-tick 70 --perf-frames 600 --death-stress
```

The headless replay of the new fixture and its presets: `./target/release/floor01_scene --headless-floor01-smoke --fixture cp03_goblin_pileup --gore-preset 1` (and 2, 3, 4) all end in `hash=6a82811d8f31c475`; `bash scripts/ci_floor01_combat.sh headless` checks it with every other fixture.

## Rights/Provenance

Code, probe, fixture, documentation and evidence by Claude (medium tier, execution of the DEATH-E1 work order); no new art was drawn and every pixel in the captures is committed GOB-02-DEATH, GOB-02B-DEATH, GOB-01, Carl, Donut and Floor 1 art under the owner-reported `ART-PUBLICATION-ALL-V1` scope, unchanged here. Independent top-tier review: pending.
