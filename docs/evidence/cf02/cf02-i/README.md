# CF02-I evidence: acting integration

The final CF02 frames (ART-CARL-CF02, ART-DONUT-CF02, ART-GOB-CF02 and the goblin slammed sheets) and the Q69 to Q72 overlays, drawn in the real Bevy scene under `cf02_rules`. Presentation only: no event, hash or input timing changes (proof below). Design and frame map: [CF02-I acting integration](../../../03-tech/floor01-scene.md#cf02-i-acting-integration). Game feel is not judged; nobody has played it.

## Real Bevy 1x GIFs

Each GIF replays a committed `cf02_*` fixture in the real render loop (lighting on, gore preset 4, `--scale 1`: 640x360 virtual pixels, no scaling), cropped around the actors (the crop is a window of the 640x360 view, 192x108 unless the script says otherwise), one frame every 2nd tick, played in real time (3/100 s per frame, the last frame held). The `_strip.png` shows the same frames side by side for a still review and the `_states.txt` lists the `COMBAT_ANIM_STATE` line of every frame (`acting_carl=`, `acting_donut=` and `acting_goblins=` give the CF02 frame drawn, as `tag:atlas frame`; `-` means the legacy or locomotion frame). Reproduce with `bash scripts/make_cf02_i_evidence.sh` (needs `xvfb-run`, ImageMagick, a debug build and the exported atlases).

| Clip | Fixture and ticks | What it shows |
| --- | --- | --- |
| [`carl_stomp.gif`](carl_stomp.gif) | `cf02_stomp_standing`, 4..40 | the stomp: knee-up tell, the stamp held through the hit-stop, the recover (frames 0, 1, 2 of the Carl sheet) |
| [`carl_follow_kick.gif`](carl_follow_kick.gif) | `cf02_confirm_kick`, 14..52 | the follow-through from a jab: the approved kick frames without the chamber (k0, k2, the contact frame k3 on the hit tick) |
| [`carl_air_kick.gif`](carl_air_kick.gif) | `cf02_air_kick_jump_in`, 0..32 | the jump-in kick: chamber, boot, then the locomotion fall frame |
| [`carl_dive_and_heavy_landing.gif`](carl_dive_and_heavy_landing.gif) | `cf02_dive_whiff_recovery`, 14..52 | the dive tuck, the plunge with the arms out, and the heavy landing for the 10 recovery ticks of a whiff |
| [`carl_bailout.gif`](carl_bailout.gif) | `cf02_bailout_escapes_swing`, 26..66 | the bail-out: push-off, the backpedalling scramble with eyes on the goblin, then the locomotion landing |
| [`carl_flinch.gif`](carl_flinch.gif) | `cf02_llama_bolt_stops`, 86..106 | the flinch (8 ticks) after a llama glob passed within 12 px of Carl without hitting |
| [`donut_startle.gif`](donut_startle.gif) | `cf02_wall_slam_kill`, 30..52 | Donut's startle (puff, then settle) when a goblin is slammed into the wall within 160 px |
| [`donut_smug.gif`](donut_smug.gif) | `cf02_assist_pounce_staggered`, 56..86 | Donut's smug (20 ticks) once her pounce on a staggered goblin has landed and she stands idle (she stands behind Carl in this fixture, so the frame is hard to read; a clearer shot is a post-F item) |
| [`goblin_hesitate.gif`](goblin_hesitate.gif) | `cf02_morale_witness_hesitate`, 14..46 | the double take of a goblin that sees a kicked body fly (20 ticks) |
| [`goblin_panic.gif`](goblin_panic.gif) | `cf02_morale_spectacle_panic`, 38..72 | the panicking goblin: arms up, looking back, running the way it flees |
| [`goblin_taunt_punished.gif`](goblin_taunt_punished.gif) | `cf02_showoff_taunt_punish`, 6..52 | the show-off's chest thump and beckon, then a jab on tick 39: the taunt frame is held 6 ticks over the hit |
| [`slammed_stomped.gif`](slammed_stomped.gif) | `cf02_kick_then_stomp`, 66..98 | the stomped finish: the fold around the stamp for 8 ticks, then the dead bug |
| [`slammed_wall.gif`](slammed_wall.gif) | `cf02_wall_slam_kill`, 24..58 | the wall crumple: the squash on the wall while the body falls, then the slump |
| [`slammed_flung.gif`](slammed_flung.gif) | `cf02_lethal_kick_far`, 60..100 | the flung finish: the GOB-01 knockback flight, the face-plant for 8 ticks from the landing, then the heap |

**NOT RUN yet (needs CF02-F):** the Carl frustration clip and every clip in the microarena. No existing `cf02_*` fixture has three whiffs in a row, so the frustration frames are proven by the unit tests (`three_whiffs_in_the_window_frustrate_carl_once_per_cooldown` and the two after it) and drawn by no GIF; they are made once the F microarena merges.

## Parity proof (tests, `src/combat_anim/cf02/tests.rs`)

| Claim | Test |
| --- | --- |
| Every `cf02_*` fixture of every registry (CF02, Donut, COMPANION) gives the identical event log, final hash and hash on every tick with the overlays on, with `q69_acting_overlays` off and with no acting art at all | `acting_overlays_on_and_off_change_no_event_hash_or_input_timing` |
| No simulation module reads `q69_` to `q72_`, `acting_overlays`, `crate::acting` or `combat_anim` | `no_simulation_module_reads_an_acting_knob` |
| Legacy state lines carry no acting text (every committed legacy line is byte-identical); the CF02 art is read only under `cf02_rules` | `legacy_fixtures_draw_no_acting_frame_and_keep_their_state_line` |
| Selector priority death > hurt > move > morale reaction > overlay > locomotion for goblins; Carl's overlay is ended (not paused) by a hurt or by running; the core's own priority tests | `the_goblin_layers_follow_q69`, `a_hurt_carl_loses_the_overlay_and_it_does_not_come_back`, the `q69_*` tests in `src/acting.rs` |
| A frame for every state: stomp, follow-through (no chamber, contact frame on the hit tick), air kick, dive, dive whiff landing (exactly 10 ticks, none after a hit), bail-out, flinch (8 ticks, idle only, judged after the miss, a swing or glob that hits is not a flinch), frustration (24 ticks, cooldown 600, a hit resets, a slow run does not count, it waits for idle inside its window), Donut startle (distance, cooldown, idle only) and smug (waits for idle), goblin hesitate, panic (mirrored to the flee direction), taunt and the 6 tick taunt hold, the three slammed finishes | `the_stomp_*`, `the_follow_kick_*`, `the_air_kick_*`, `the_dive_*`, `the_bail_out_*`, `a_dive_whiff_*`, `flinch_and_frustration_frames`, `a_near_miss_*`, `a_glob_*`, `three_whiffs_*`, `frustration_waits_*`, `donut_*`, `an_overlay_waits_*`, `goblin_hesitate_panic_and_taunt_frames`, `a_non_launching_hit_*`, `a_punished_taunt_*`, `the_three_slammed_finishes_*` |
| Every CF02 move and reaction frame is drawn by a real fixture run | `real_runs_draw_every_cf02_move_frame` |
| The exported atlases agree with the grid sources (fails under `REQUIRE_EXPORTED_ATLAS=1` when the export is missing) | `the_exported_atlas_agrees_with_the_grid_source_when_present` |

Rights and provenance: the images show only the committed original Carl, Donut and goblin art under the recorded scope in the [rights register](../../../04-production/rights-and-release.md). No new art. Model attribution: Claude Sonnet (this PR; execution and integration tier).
