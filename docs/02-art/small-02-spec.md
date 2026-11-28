# SMALL-02: Floor 1 small follow-ups bundle (spec, adopted defaults and open questions)

**Status:** implemented as one PR, owner art review and game-feel review pending. **Reuse only:** no new key, no new pixel, no palette change. Presentation items never touch the simulation: the window hash equals the headless hash on every still (`docs/evidence/small-02/capture_lines.txt`), `MANIFEST_HASH` and every golden are unchanged.

Model routing for the Rights/Provenance line of the PR: executed by Claude Sonnet, to be reviewed by Claude Opus (a separate agent).

## 1. Items and results

| Item | Result |
| --- | --- |
| MORDECAI-DRIVE (ACT1-ART-Q2) | Done. `refuge::Mordecai::frame_at` loops the atlas `idle` tag (frames 0 to 3, 40, 22, 14 and 22 ticks, read from the atlas frame durations) on the presentation tick, and shows the `talk` key (frame 4) while Carl is within 48 px of his column. The review scene `--scene refuge_guild` stays on frame 0 (its presentation tick is pinned to 0). |
| CINE-REDUCED-MOTION | Done. `PresentationSettings::with_reduced_motion`; `Args::cinematic_settings` applies `--hud-reduced-motion` last, so reduced motion drops the slow motion and the camera emphasis even over `--cinematic-finishes`. Hit-stop, death VFX and every gameplay tick are unchanged. Tests in `cine.rs` and `floor01_scene.rs`. |
| REST-Q10 | The recorded default is "decay runs" (REST-AD14, `f01-playtest-ready-01.md`), so nothing is implemented. The docs now say the data is honoured as decided, and `quiet_rooms_do_not_freeze_audience_decay` pins the first decay on the 480th award-free step. |
| ROOM-PILOT-RUN | Skipped, SMALL02-Q1. |
| R4-EXIT-OPEN | Not reproducible (20 of 20 byte-identical captures, 4 of them in parallel). The doc note is updated precisely; no code change. |
| ST-OQ1 | Done (SMALL02-AD2). |
| HC-OQ3 | Done (SMALL02-AD3). |
| HEAT-FU-1 | Decided (SMALL02-AD4): the M0 spit trace is dropped; no placement change. |
| LLAMA-03-FU | Skipped, SMALL02-Q2. |
| ENG-02 tail | No presentation-only part is left; recorded as SMALL02-Q3. |
| Stale comments | `src/f01_rest.rs` and `src/bin/floor01_scene.rs`: comments only (no printed string, no data). |

## 2. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| SMALL02-AD1 | Mordecai's `talk` key shows while Carl is within **48 px** of his column; otherwise the `idle` loop runs at the atlas durations. The trigger is Carl's position, not a text beat. | The guild's lines are HUD messages that the scene does not read back, so "a text beat is on screen" would need a new observer; a radius is the simpler of the two options in the work order. Carl (the lead hero) is the only trigger; Donut's position does not count. |
| SMALL02-AD2 | A landing plume (`danger_fx_art::landing_dust`) is cropped at the nearest **other prop** whose hit target overlaps the plume's 20 row band and begins within its reach, and it is **dropped** when that prop stands flush against the remnant. In the Boiler the drum is flush, so the right plume is dropped and the left plume is unchanged. | ST-OQ1's two suggested options in one rule. The plume that merged with the drum's orange band is the one that disappears, as dust would be stopped by the drum. |
| SMALL02-AD3 | A counted hit on a J-B stack (`PropHit` of its mass, so a hit refused while she is not out draws nothing, HC-AD8) tints the post `FLASH_TINT` for **4 ticks** and plays one ENV-02 `timber_splinter_burst` (4 frames of 4 ticks) centred on the post, **30 px** above its foot, in front of the actors like the corral partition's. A restart ends it. | Reuses #320's burst and the room's existing flash tint; no gore. |
| SMALL02-AD4 | HEAT-FU-1: the M0 wall spit trace is **dropped** (the spec's own third option); `heat_scorch_wall_c` stays unplaced. | Every unpooled brick of M0 is inside a torch pool (HEAT-AD12 forbids a decal there); above the head band (rows 524 to 560) the trace's two `flame` pixels would sit at rows 507 to 509, against the rule that embers stay at y 552 or lower; inside the band its `brick` 4 rim is darker than rule 10.3 allows. Placing it needs two rule exceptions, which is more than a default. M0 keeps its ash drift and the floor spit traces. |

## 3. Open questions

| ID | Question | Default until answered |
| --- | --- | --- |
| SMALL02-Q1 | ROOM-PILOT-RUN: how should a piloted `--f01-run` cross a room boundary? `--f01-run` relaunches the binary per room and keeps every other flag, and each pilot kind (`--room-pilot` plans, `--ring-pilot`, `--rest-pilot`, `--act1-pilot`, `--trog-pilot`) is valid in one room only, so a run would need a per-room pilot table and a rule for the rooms that have none. That is a design change to the run relaunch, not a small gate. | The refusals stay: a pilot is a one-room evidence tool; the run is walked by a human. |
| SMALL02-Q2 | LLAMA-03-FU: the limp ear of the llama's recover frames reads as a collar. | Not changed. The fix is a pixel edit of reviewed llama frames, which routes to the top tier ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)); no edit was made blind. |
| SMALL02-Q3 | ENG-02 tail: death frame 2's pot origin is already (28, 40) (ENG2-X5, applied in #300); the ram cycle `[12, 20, 21, 20]` needs a drawn frame for the other leg; the forced `slammed_wall` clip is the death lab's forced pose, not a runtime choice. | None of these is a presentation-only change; the ram frame is a new key. |

## 4. Evidence

`docs/evidence/small-02/` holds one 640 x 360 window still per visual item, each captured twice (byte-identical) with the window hash equal to the headless hash of the same tick: `scene_mordecai_idle_0`, `scene_mordecai_idle_2`, `scene_mordecai_talk`, `scene_boiler_plume` and `scene_hoarder_post_hit`, with `capture_lines.txt` and `SHA256SUMS`. There is no still for the spit trace (dropped) or the llama ear (skipped). Regenerate with `bash scripts/make_small02_evidence.sh` (the debug binary, `bash scripts/export_assets.sh`, `xvfb-run`, ImageMagick).

## 5. Tests

- `refuge::tests::mordecai_frame_follows_the_loop_and_the_talk_radius`, `mordecai_loads_from_the_exported_atlas` (the idle ticks 40, 22, 14, 22 and the talk frame 4).
- `cine::tests::reduced_motion_drops_the_cinematic_finishes`, `floor01_scene::tests::cinematic_finishes_is_on_for_live_play_and_off_for_automated_runs`.
- `playtrace::tests_f01_late::quiet_rooms_do_not_freeze_audience_decay`.
- `danger_fx_art::tests::the_boiler_weight_throws_its_landing_dust_on_the_landing_tick` (the right plume is dropped at the flush drum, the left one is whole), `a_restart_clears_the_gate_drop_and_the_slam_dust`.
- `hoarder_art::tests::a_counted_post_hit_flashes_and_bursts_for_a_few_ticks`.

## 6. Living-horizon note

None of these items changes a playable move, a progression gate or a floor mechanic, so `docs/01-design/character-progression-horizon.md` still fits unchanged.
