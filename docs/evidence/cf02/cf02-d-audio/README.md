# CF02-D and CF02-AUDIO evidence

Stacked on #149 (CF02-B and CF02-G): it reads B's `BodyContact`, `WallSlam` and G's events, so it must merge after it. Replays and tests, not captures: the death presenter and the cue overlay are presentation only, so no authoritative byte changes and no replay golden moves.

## Defeat language (CF02-D, Q21, Q22, Q36, Q43)

Switches (all read through `Cf02Tuning::on`, so only under `cf02_rules`): `q36_death_overlay`, `q36_gore_budget`, `q36_cue_overlay`. Wiring: `DeathPresenter::with_cf02` in `floor01_scene` when `CombatState::cf02()` is set.

| Defeat (same-tick events) | Selected finish (cf02 overlay) | Pose | `--demo` / legacy |
| --- | --- | --- | --- |
| lethal `Hit{Stomp}` or `Hit{DiveStomp}` (heavy) | `goblin_slammed` | stomped | `goblin_knockback_down` |
| `WallSlam` of that goblin, then its defeat (heavy) | `goblin_slammed` | wall crumple | `goblin_knockback_down` |
| lethal `Hit{PushKick}` or `FollowKick` (heavy) | `goblin_slammed` | flung | `goblin_knockback_down` |
| routine blunt kill (jab, haymaker) | `goblin_knockback_down` | none | `goblin_knockback_down` |
| block, edge, point blank blast | `goblin_crushed`, `goblin_decapitated`, `goblin_exploded` | none | unchanged |

Gore baselines at each preset (pieces thrown on the defeat tick; the arm is a separated part; "legacy" is the same kill without `--cf02`). Counts are pinned by `q43_each_slammed_pose_has_its_own_gore_baseline_at_preset_four`, `q43_presets_one_to_three_scale_by_their_existing_ratio` and `q22_preset_four_heavy_and_catastrophic_finishes_break_up_more` in `src/death_present.rs`.

| Finish | Preset 4 | Preset 3 | Preset 2 | Preset 1 | Legacy preset 4 |
| --- | --- | --- | --- | --- | --- |
| stomped | 5 gibs (6 quarters), no arm | 2 gibs, no arm | none | none | 3 gibs |
| wall crumple | 3 gibs (4 quarters) + 1 arm now | 2 gibs + 1 arm | none | none | 3 gibs |
| flung | 5 gibs + 1 arm on landing | 2 gibs + 1 arm | none | none | 3 gibs |
| crush (catastrophic, Q22) | 5 gibs | 1 gib (unchanged) | none | none | 3 gibs |
| decapitation (Q22) | head + 1 arm | head (unchanged) | head | head | head |
| explosion (Q22) | 15 gibs (6, 6, 3) | unchanged | unchanged | unchanged | 10 gibs |
| routine blunt kill (Q43) | + 1 transient spray (6 drops, gone after 24 ticks, no decal) | unchanged | unchanged | unchanged | none |

Presets 1 and 2 are byte-identical to the legacy presenter for the same kill (test `q43_presets_one_to_three_scale_by_their_existing_ratio` compares every piece). Preset 3 draws the new finishes by Q43's existing-ratio rule (a stomp: 2 gibs against the legacy kick's 1).

Parity: `q21_q36_real_cf02_runs_pick_the_poses_and_keep_every_hash` replays every non-boss `cf02_*` fixture at presets 1 to 4 with the overlay on and off and requires the identical state hash and event log each time, that the legacy presenter never reports a slammed pose, and that every slammed pose equals `slam_pose_for` over its own tick's events (the three poses all occur, `cf02_wall_slam_kill` is the wall crumple). `q36_a_tuning_with_the_rules_off_changes_nothing` pins that a tuning with the rules off leaves the presenter digest equal to the plain one.

## Cue overlay (CF02-AUDIO, Q24, Q27, Q36)

Five original cues, three takes each, in `assets/audio/floor01/cf02/` (see its [README](../../../../assets/audio/floor01/cf02/docs/README.md) and [QA report](../../../../assets/audio/floor01/cf02/docs/QA_REPORT.md): 15 files, all structural checks pass, levels inside the Floor 01 band). The mapping, reuse list and rules are in [`docs/03-tech/audio.md`](../../../03-tech/audio.md#cf02-cue-overlay-cf02-audio-combat-fun-02-q24-q27-q36). A listening aid with the first take of each cue in order: `assets/audio/floor01/cf02/previews/cf02_cues_preview.mp3`.

| Acceptance | Test |
| --- | --- |
| each Q24 cause has its own cue | `q24_each_cause_has_its_own_cue_and_the_reuse_map_holds`, `q24_the_cf02_fixtures_emit_the_overlay_cues` (real fixtures sound the punch, stomp, wall slam, heavy defeat and body contact cues) |
| overlay off = DEMO-01 sound | `q36_overlay_off_is_the_legacy_cue_map_for_every_event` (rules off, either switch off), the global `cue_map.json`, `audio_events.json` and manifest are untouched (the existing 38-file test) |
| overlap limit holds | `q24_the_overlap_limit_holds_for_the_overlay_cues` (pack mixer, overlay value and Q24 knob all 2; a third overlapping major transient is dropped) |
| no cue drives gameplay | `q24_no_simulation_file_reads_the_audio_module` (no file outside the audio module, adapter and binaries names it) |
| manifest, checksums, provenance | `the_cf02_overlay_manifest_matches_its_files_and_every_cue_resolves`; `python3 scripts/author_cf02_audio.py --check` |

## Not run

- Contact sheets per pose at presets 1 and 4 and the `--demo` against `--cf02` side-by-side of a lethal kick: **NOT RUN as in-engine captures**. The goblin slammed sheets (ART-GOB-CF02) are not wired into any binary until CF02-I, so the sprite is still the GOB-01 knockback; the pose and gore differences above are pinned by tests. The art's own contact sheets are in the ART-GOB-CF02 evidence.
- Listening: **NOT RUN**. No human has heard the five cues (design section 8, item 3).
- A dive that lands without hitting is silent (no simulation event exists for it; this PR adds none).
