# CORRAL-D: the rush-breakable partition of room 0

**Status: merged, game-feel review pending** once the PR merges. Nothing here is accepted: the owner judges how the screen reads and plays (it makes Donut's missile useless across the pen until B's rush or a hero breaks it) by playing it. The art is the AREA-HEAT-01 prototype (owner art review pending).

Room 0 now has the optional timber partition of the sheet (`docs/01-design/enemy-gameplay-llama-rush.md` section 10, CRL-Q30, DV-9, DV-10) **on by default in `--scene f01_late --f01-run`** (and in `--f01-room corral`): a 16 x 48 solid at x 1392..1408 that blocks bolts and globs, that both heroes jump or stand on, and that breaks into its 16 px remnant on a rush contact (same tick) or on the third distinct hero hit.

Every run below is a scripted pilot or a fixture, never a human. Window runs are xvfb runs of the debug binary at `--scale 1`, with the window's final `COMBAT_STATE` hash equal to the headless run's ([`captures.txt`](captures.txt), five `match=yes` lines).

## What was built

| Part | Where |
| --- | --- |
| Sidecar prop data: `pen_partition` (`kind: partition`, target 1392..1408 x 532..580, remnant 1392..1408 x 564..580, `partition: { break_hits: 3, shove_min_impulse_px_s: 160 }`) | `assets/levels/floor01/f01_corral.encounter.json`; the validator (`PropKind::Partition`, a typed `PartitionConfig`, the geometry rules, unknown knobs refused) in `src/encounter.rs` |
| The machine: `Partition` (`Intact`, `Weakened`, `Broken`), hits counted once per move instance, a shove at or above the threshold breaks it at once, everything else `NotAccepted` | `src/env_prop.rs` (appended, `partition_tests`) |
| The env step: the machine, the `PartitionWall` and `Remnant` solids synced as `Material::Timber` (KR6), the remnant-lift rule on the break tick, the hash and the state-line token | `src/combat_scene/env.rs` |
| The rush contact: the llama world's `partition` index is the synced wall solid; on `LlamaRushEnd { end: Partition }` the same tick sends the prop a `Stimulus::Shove` at the rush's 240 px/s | `src/combat_scene/corral.rs` |
| Hero hits: Carl's jab, haymaker and kick through `strike_env`, Donut's swat and pounce through `env_donut_melee`; Magic Missile, a glob, a launched body and a blast do nothing | `src/combat_scene/env.rs` |
| The event: `PropPhase::Broken` (`partition_broken`), appended; the existing wall-break cue | `src/combat.rs`, `src/audio.rs` |
| The presenter: the frame of the prop's state from `floor01_heat_partition` at (1380, 532) | pure half `src/heat_kit.rs` (`partition_piece`, `partition_origin`, `partition_atlas_rects`), Bevy half `src/bin/floor01_scene/corral_art.rs` (`sync_partition_art`) |
| CORRAL-AUTO variants `absent`, `intact`, `broken` and the partition-top and remnant-top starts | `src/corral_eval/` (policy version 3) |
| The evidence plan `--room-pilot corral_partition` | `src/room_pilots.rs` |

## Fixtures

The seven fixtures of sheet section 15 are `src/corral_eval/partition_fixtures.rs`, on the evaluator's real-party rig with the room's real sidecar prop. Their replays (COMBAT_EVENT lines and the final hashes) are in [`fixtures/`](fixtures/), regenerated with `CORRAL_D_EVIDENCE_DIR=<dir> cargo test --lib corral_partition_fixtures_write_their_replays_when_asked`.

| Fixture | Result |
| --- | --- |
| `corral_partition_rush_break` | B (tell 33, commit 65) rushes a Donut at 1376: tick 78 `LlamaRushEnd { Partition }`, `PropHit { Shove }` and `PropPhase { broken }` on that same tick, B bonked at pivot 1436, no `HeroHurt`; the remnant stands and the screen's solid is off at the end of the tick; a bolt then reaches B (tick 84) |
| `corral_partition_near_miss` | B's leash minimum set 4 px short (1440): the rush ends `Miss` at 1440, no prop event, the screen still stands 600 ticks on |
| `corral_partition_carl_breaks` | one jab counts one hit (progress 1, weakened), two more break it (progress 3, one `broken`); a further jab on the remnant counts nothing. `corral_partition_donut_pounce_counts_the_same_way`: three pounces |
| `corral_partition_missile_blocked` | eight bolts (all 8 mana spent): the screen stays intact with 0 hits and no prop event, B unhurt |
| `corral_partition_glob_blocked` | with the screen B does not spit at the shelf Donut (the glob path is not clear, CRL-Q11) where with no screen it spits and hits her; a glob forced out at B's mouth dies on the face, hurts nobody and is no stimulus |
| `corral_partition_remnant_bolt_passes` | authored pre-broken: the remnant is 1392..1408 x 16 px and a floor bolt passes over it to B |
| `corral_partition_retry_restores` | a restart after the break restores the standing screen, its wall solid at once and the env hash bit for bit |
| `corral_partition_respawn_keeps_broken` (CD-AD10) | a checkpoint respawn keeps a weakened or broken screen; with B placed in the old wall column no wall is rebuilt around it, B moves off the column and the screen never stands again; a restart then restores it |

Also pinned: `corral_partition_every_named_hero_move_counts_one_hit` (Carl's jab, haymaker and push kick; Donut's pounce, and her swat through the hit-box intake), `a_checkpoint_respawn_keeps_the_broken_partition_in_the_corral_rush_run`, `corral_partition_is_jumpable_and_standable_by_both_heroes` (DV-10: Carl and Donut each cross it, and stand on its 48 px top), `..._a_hero_on_top_drops_onto_the_remnant_when_it_breaks`, `..._break_lifts_a_hero_that_overlaps_the_remnant`, `heat_kit::tests::the_partition_art_core_sits_exactly_on_the_collider` and `room_pilots::tests::the_corral_partition_plan_shows_the_screen_standing_weakened_and_broken`.

## In-scene stills and the break sequence (1x)

Plan `corral_partition`: Carl clears M0 and M1, walks to the face of the screen and jabs it every 24 ticks; B wakes and rushes. Tick 652: the first jab is counted (weakened); tick 673: B's rush bonks the screen (broken).

| File | What it shows |
| --- | --- |
| [`partition-640x360-tick640-intact.png`](partition-640x360-tick640-intact.png), [`partition-640x400-tick640-intact.png`](partition-640x400-tick640-intact.png) | the standing screen (the `intact` frame), Carl at its face, B behind it |
| [`partition-640x360-tick660-weakened.png`](partition-640x360-tick660-weakened.png), [`partition-640x400-tick660-weakened.png`](partition-640x400-tick660-weakened.png) | after the first counted jab: the `weakened` frame (snapped plank, cracked board, snapped brace) |
| [`partition-640x360-tick680-broken.png`](partition-640x360-tick680-broken.png), [`partition-640x400-tick680-broken.png`](partition-640x400-tick680-broken.png) | after the rush: the 16 px remnant, B dazed (the LLAMA-01 hurt frame, then blown) with the bonk dust |
| [`partition-break-640x360-tick646-688-x1.gif`](partition-break-640x360-tick646-688-x1.gif), [`partition-break-640x400-tick646-688-x1.gif`](partition-break-640x400-tick646-688-x1.gif) | the weakened-to-broken sequence, every 2nd tick 646..688 (22 frames) |
| [`partition-break-x3-sheet.png`](partition-break-x3-sheet.png) | six frames of it cropped around the screen and B, 3x |

[`f01-run-smoke.txt`](f01-run-smoke.txt): a bare `--scene f01_late --f01-run` prints `partition=1392..1408_48px` on the room's entry line and, in a window, `CORRAL_PARTITION drawn at (1380, 532)`.

## CORRAL-AUTO (policy version 3)

The corral gate on the final head, 53,652 runs (the baselines included): **pass**. The partition variants run on the M2 stage and every start group is judged per variant ([`report.md`](report.md), [`summary.json`](summary.json), `runs.csv` not committed; about 33 s of debug run time on 8 worker threads).

| Variant | Runs | KC19 groups | KC19 failing | CRL-Q35 breaches | Partition breaks (rush / hero hits) |
| --- | ---: | ---: | ---: | ---: | ---: |
| absent | 15,864 | 2,122 | 0 | 0 | 0 / 0 |
| intact | 16,536 | 2,230 | 0 | 0 | 2,206 / 0 |
| broken | 15,732 | 2,108 | 0 | 0 | 0 / 0 |

KC19 on the decisive variant 6,460 groups, 0 failing; the CRL-Q35 invariant, the M1 positive control, a mobile Donut, a Carl melee and a 2P co-op victory, and the hard system failures all `pass`; no invalid start. The `absent` row is the CORRAL-C sweep unchanged (2,122 groups, as in `../corral-c/`). The sheet's reach claims hold on the new surfaces: nothing a Donut does from the 48 px top of the intact screen reaches a llama (M2-06, 732 parked runs, 0 clears), and from the 16 px remnant top she reaches only B (84 runs, 42 clears of B, none of A). The STATIONARY-DONUT gate (KC3) is not broken.

## Moved pins and what stayed

| Pin | Before | After | Why |
| --- | --- | --- | --- |
| `f01_corral::ENCOUNTER_HASH` (the `f01_corral_encounter` data pin) | `0x7161_1549_3c07_e42f` | `0xc642_7d89_07c3_b8a7` | the sidecar gains its prop (and the status text) |
| The room 0 combat state hash, env hash block and state-line token | no prop | the partition's state is mixed (`env=0:partition_Intact`) | a new machine in the room; every room-0 replay moves |
| `corral_room_pilots_pin_their_final_hashes_at_seed_0` and `corral-f/pilot_hashes.txt` | carl_solo gate 863, exit 971; donut_solo 862, 973; companion 1017, 1125 | carl_solo 1006, 1109; donut_solo 1202, 1313; companion 981, 1092 (all undefeated) | B cannot walk past the screen and the bolts it blocks make the scripted fights run later (Donut solo by 340 ticks); the four hashes are re-pinned, `corral_rush` ends with 1 death (was 2) and its hash is `1100063bb81c0a27` (CD-AD10: the screen stays broken across its respawns) |
| `the_corral_rush_plan_draws_the_llama_rush_tick_map` | tell 742, contact 791 | tell 634, commit 666, bonk on the screen 673 | the `corral_rush` plan's first rush now ends on the screen |
| CORRAL-AUTO `POLICY_VERSION` and the gate counts | 2; 2,122 groups; 47 grid-only columns | 3; 6,460 groups; 51 | the variants and the four partition-top and remnant-top grid starts |

Unchanged and verified: `MANIFEST_HASH`, `TABLE_HASH`, every legacy golden and every other room's hashes (the full test suite and `scripts/ci_floor01_combat.sh headless` pass), the evaluator's `absent` variant (its run ids and group counts are the CORRAL-C ones), and the replay and capture hashes of every other room. The older [`corral-f/`](../corral-f/README.md) captures, session lines and stills are the record of the pre-partition head (noted there).

## Adopted defaults (CD-AD*)

| ID | Default | Why |
| --- | --- | --- |
| CD-AD1 | The sheet's collider (1392..1408 x 532..580) is authoritative and the art needs no change: the 40 x 48 canvas holds its solid core in columns 12..27 by spec, so it is placed at the collider's top-left minus (12, 0) = (1380, 532), the anchor HEAT-AD1 names. The margin (rope tails, straw, debris, the 4-row sill) is visual. A test pins the core to the collider and the remnant's flat top (canvas row 32) to the sidecar remnant | A 16 px collider under a 40 px drawing reads as a screen with a footing, not as a wider wall; nothing conflicts |
| CD-AD2 | A counted hero hit is one hit whatever the move's support-hit weight (the reinforced haymaker, weight 2, still counts one): "three distinct hits" | CRL-Q30 |
| CD-AD3 | The slowest shove that breaks it is 160 px/s (the other props' launch and shove threshold, BOILC-AD2); the rush is 240 px/s | an enemy shove under that is not a bait |
| CD-AD4 | A rush that meets the right face stops with its pivot at 1436 (box front 28 px), not the 1428 of the geometry table (the walking stop); the near-miss fixture's leash minimum is therefore 1440 | measured; the table's 1428 is the walk |
| CD-AD5 | The break is the appended `PropPhase::Broken` (`partition_broken` of #181); the event order on the contact tick is the rush end, then `PropHit { Shove }`, then `PropPhase { broken }` | a new variant is appended, no hash moves |
| CD-AD6 | A launched body (a kicked goblin) and a blast do not break it | the sheet names only the rush and physical hero hits |
| CD-AD7 | The CORRAL-AUTO `broken` variant is a tick-0 setup write (exempt like the other setup writes, F11); the partition-top and remnant-top starts are judged on the pivot column alone (her body overhangs the 16 px top); a start exists only in the variant that has its surface | the sheet's M2-06 and M2-G rows |
| CD-AD8 | A new evidence plan `corral_partition` (Carl jabs the screen) shows the three states in the run; the `corral_rush` plan is unchanged apart from its re-based tick map | the three states cannot all be shown by the rush alone |
| CD-AD9 | The break plays the existing wall-break cue; no splinter or dust VFX of its own (the bonk dust of the rush stand-in plays) | cue reuse, no new art or sound |
| CD-AD10 | A checkpoint respawn keeps the partition's state (a broken or weakened screen stays so); only a restart (retry) restores it. The #298 review reproduced the defect of restoring on respawn in the `corral_rush` run: B at 1412 inside the rebuilt wall, frozen, spitting and rushing through it | A respawn does not send a llama home (it keeps its position), like a blown charge stays blown and a spent sticker spent; the sheet only requires a reset on retry |

## Open notes (non-blocking)

- **Feel (the owner's call).** The screen makes Donut's missile useless across the pen: a Donut at the entry cannot hurt B until the screen is broken (by B's rush, or by Carl, or Donut's own swat). The scripted Donut-solo clear takes 340 ticks longer. Whether the bait reads and whether the break feels earned is a play question (sheet section 8).
- **Splinter VFX.** The sheet leaves the break's splinters and dust to CORRAL-D as presentation; only the existing cue and the llama's bonk dust play. A dedicated burst is a follow-up if the owner wants more spectacle.
- **Companion AI.** The partner AI treats the standing screen like any 48 px solid and the four route pilots (including the `cf02_free` companion) clear the room; nobody was stuck over the limit in any of the gate seeds. A human playtest of a real partner is not run here.
- **Debug build only.** Windows and timings are debug-build, xvfb runs; no frame-time measurement was made (the machine is a few rectangles and one sprite).
