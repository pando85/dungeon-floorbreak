# COMBAT-FUN-02 evidence

Index of the COMBAT-FUN-02 evidence. CF02-00 created it and CF02-REVIEW (this revision) owns it. Each stage wrote its captures and notes only in `docs/evidence/cf02/<pr-id>/`; the one shared exception is [`replays/`](replays/README.md): the committed replay of every `cf02_*` fixture (`headless_<fixture>.txt`), append-only, because the frozen CI script reads it.

Status of the whole feature: **merged on main and re-verified headless; game feel is not accepted.** Nothing here is a human verdict. The owner plays the room next: see [`owner-handoff.md`](owner-handoff.md) and fill in [`playtest-scorecard.md`](playtest-scorecard.md).

## Owner documents (CF02-REVIEW)

| Item | What it is |
| --- | --- |
| [`owner-handoff.md`](owner-handoff.md) | Exact launch commands (solo, one player with the assistant, two pads, keyboard), the controls, what to judge in each of the three approaches, the open adopted values the owner decides, and the **NOT RUN** list |
| [`playtest-scorecard.md`](playtest-scorecard.md) | The scorecard to fill in per session, with the build SHA and flags pre-filled |
| [`scorecard-template.md`](scorecard-template.md) | The CF02-00 template the scorecard extends (kept as written) |

## Evidence by stage

| Stage (PR) | What it adds | Evidence |
| --- | --- | --- |
| CF02-00 baseline (#134) | Q23 audit of B2 contact, B3 decapitation, B5 blast, B4 llama finishes and the camera traces; the knob schema, the `--cf02` and `--cf02-seed` flags, the COMPANION stubs | [`gap-report.md`](gap-report.md), [`audit/audit_events.md`](audit/audit_events.md), [`audit/camera_trace.md`](audit/camera_trace.md), [`captures/`](captures/) (fourteen 640x360 frames with `SHA256SUMS`) |
| ACT-DESIGN (#133) | Acting key poses and the acting spec (Q69 to Q74) | [`../cf02-act-design/`](../cf02-act-design/README.md) |
| CF02-CORES (#139) | Pure morale, input buffer and acting cores | Unit tests only (no wiring, no hash change) |
| CF02-C1 (#144) | Llama targets: Donut's swat interrupts, the light guard, the bolt stops at the llama (Q18, Q37, Q44) | Five replays (`cf02_llama_*`, `cf02_donut_solo_llama`) in [`replays/`](replays/README.md) |
| CF02-A + CF02-H (#147) | Carl's stomp, follow-through, lying goblin, Donut's hold; air kick, dive, bail-out, input buffer and cancel table | [`cf02-a/`](cf02-a/README.md) (13 fixtures), [`cf02-h/`](cf02-h/README.md) (14 fixtures, latency table) |
| CF02-B + CF02-G (#149) | Contact knockdown, bowling bodies, wall slams; goblin morale and reactions | [`cf02-b/`](cf02-b/README.md) (9 fixtures), [`cf02-g/`](cf02-g/README.md) (11 fixtures) |
| CF02-D + CF02-AUDIO (#154) | The defeat language (slammed finishes, gore budget) and five audio cues | [`cf02-d-audio/`](cf02-d-audio/README.md); cues, QA report and previews in `assets/audio/floor01/cf02/` |
| ART-CARL-CF02 (#146), ART-DONUT-CF02 (#142), ART-GOB-CF02 (#148) | The three acting sheets (13 Carl frames, 3 Donut frames, 22 goblin frames) | [`../carl-cf02/`](../carl-cf02/README.md), [`../cf02-donut/`](../cf02-donut/README.md), [`../cf02-goblin/`](../cf02-goblin/README.md) |
| COMPANION-AI-01 01A-wire (#151) | The `cf02_assist` companion drives Donut in a solo run (Q13 to Q17, Q33, Q38 AI part, Q39, Q40) | [`../companion-ai/README.md`](../companion-ai/README.md) (12 replays and the metrics table) |
| CF02-C2 + CF02-F (#158) | Solo swap (12-tick cooldown, eased pan), Donut's bail-out, the microarena with shore, perches, parapet and llama yard | [`cf02-c2f/`](cf02-c2f/README.md): [`pan_trace.md`](cf02-c2f/pan_trace.md), `captures/` at 640x360, 640x400 and 1920x1080, six 1x GIFs, [`render_match.txt`](cf02-c2f/render_match.txt) |
| CF02-I (#159) | The acting overlays and final CF02 frames in the scene | [`cf02-i/`](cf02-i/README.md): twenty 1x GIFs with strips and per-frame state lines |
| CF02-REVIEW (this PR) | Re-verification on the final main, this index, the scorecard, the owner handoff, the Q41 cinematic count, one log-label fix, the U2 CI probe fix | This file |

## Re-verification on the final main

Base: main at `9dbf5f54ed36f399c91f00573438342873df2114` (CF02-I merged). Run on 2026-11-21 in a clean worktree on Linux x86_64, `rustc`/`cargo` 1.99.1 (local toolchain), software rendering under Xvfb where a window was needed. The head of this PR adds only the two fixes and the one test described below, plus documentation. **Re-run on the final head of this PR:** the format, clippy, test, headless-CI and demo-CI rows below were run after those changes (the headless script once more after the U2 edit); the fixture, variant and live rows ran on the same source (the label fix does not touch the simulation).

| Check | Command | Result |
| --- | --- | --- |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Clippy | `cargo clippy --locked --all-targets -- -D warnings` | exit 101 on this local 1.99.1 toolchain: six `clippy::needless_range_loop` findings, no other lint (`src/combat_scene/morale_cf02.rs` lines 372, 407 and 538, `src/combat_scene.rs` 1858, `src/hud_adapter.rs` 620, `src/autoplay/party/tests.rs` 520). Not fixed here: CI on stable is the gate and is green on the merged PRs |
| Clippy, that one lint allowed | `cargo clippy --locked --all-targets -- -D warnings -A clippy::needless_range_loop` | exit 0 (no other warning exists) |
| Tests | `cargo test --locked --all-targets` | exit 0: 1716 passed, 0 failed, 9 ignored (lib 1653, `floor01_scene` 34, others 29). `playtrace::tests::killed_writer_recovers_the_valid_prefix` passed (it failed once on an earlier main, see the backlog) |
| Headless CI | `bash scripts/ci_floor01_combat.sh headless` | exit 0: every `*_OK` line (173), including 68 `CF02_HEADLESS_OK` and 12 `COMPANION_HEADLESS_OK`, legacy fixtures at gore 1 to 4, party, boss, Donut, cameras |
| Demo CI | `bash scripts/ci_floor01_combat.sh demo` | exit 0: the six DEMO-01 beat probes and the whole route (7 `DEMO_HEADLESS_OK`) replay at gore 1 to 4 equal to the committed logs |
| Docs | `python3 scripts/check_docs.py` | passed on the head of this PR: 282 Markdown files, 9 required files (280 Markdown files and 2223 JSON fixtures before the two new documents and before `generated/art` was exported; the JSON count depends on that export) |
| Assets | `bash scripts/export_assets.sh` then `python3 scripts/validate_assets.py` | export exit 0 (LibreSprite under Xvfb); validate exit 0: 88 VALID, 0 INVALID |
| Fixtures | `--list-fixtures cf02_` (76 names, 68 plus the 8 `cf02_assist_*`) and `--list-fixtures companion_` (4 names) | 80 of 80 replay to exactly the committed `COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines and hash (table below) |
| Variants | the same 80 fixtures with `--gore-preset 1`, `--gore-preset 4`, `--camera auto`, `--camera scroll`, `--camera 1.0`, `--camera 1.25`, `--camera 1.5`, `--cinematic-finishes=on` and `--cinematic-finishes=off` | 720 of 720 runs identical to the committed replay (gore, every camera, CINE on and off never reach the simulation) |
| Acting on, off, no art | test `combat_anim::cf02::tests::acting_overlays_on_and_off_change_no_event_hash_or_input_timing` (every `cf02_*` fixture of every registry) and `no_simulation_module_reads_an_acting_knob` | pass (in the 1716) |
| Parity with the rules off | test `combat_scene::tests::cf02_parity_off`, `cf02::tests::tuning_hash_is_pinned` | pass; `TUNING_HASH` is `0xffd13ba92d65445c`, printed by the `CF02_CONFIG` line of every `--cf02` run |
| No unbounded contacts | tests `q09_a_knocked_down_goblin_never_chains_into_a_third`, `cf02_no_chain` replay, `goblin::tests::knockback_is_bounded_and_stopped_by_walls_and_the_layout_edge`; the CF02-B acceptance tests (no duplicate effect per (launch, target), no chains) | pass |
| Live smoke, autopilot | `xvfb-run -a floor01_scene --cf02 --autopilot --scale 1 --no-audio --record --record-dir <scratch> --exit-after-ticks 1800` | **The autopilot is not a live party run:** it keeps the room fixture loaded and the single-hero path, so there is no Donut and no assistant, and the clear is Carl alone (a lower bound on the room length). Exit 0, no panic. `CF02_CONFIG: rules=true seed=fixture schema_version=1 tuning_hash=0x2da48577fa64315e (the hash when this run was made; the re-review fixes of PR #149 added two knobs, now 0xffd13ba92d65445c)`, **`CF02_ARENA_COMPLETE tick=487`** (8.1 s of simulated time, 2 goblin panics on ticks 179 and 427), `PLAYTRACE_SAVED ticks=1800 status=Completed` |
| Live smoke, idle | `floor01_scene --cf02 --scale 1 --no-audio --record --record-dir <scratch>` under a private Xvfb, no input for about 20 s, then Esc | The only windowed run of the owner's configuration (live party, assistant selected), and **it was idle: Carl stood at x 128, every goblin starts at x 520 or beyond (outside their 220 px sight), so no combat happened.** Exit 0, no panic, no `CF02_ARENA_COMPLETE`, `PLAYTRACE_SAVED ticks=1203 status=Completed` (more than the 900 ticks asked for). The scripted variant (`--exit-after-ticks 900`) also exits 0 |
| Playtrace | `playtrace verify <session> --dir <scratch> --root .` on both sessions above | autopilot session: `result=PASS scope=full ticks=1800 episodes=1 combat_events=54 build=identical`; idle live session: `result=PASS scope=full ticks=1203 build=identical`. Both exit 0. The manifest keeps `replay_unverified` and `training_eligible=false`, as designed (the verifier says which fields it does not re-derive) |
| Q41, cinematics per attempt | test `cf02_fixtures::tests::cf02_room_cinematic_count_per_attempt` (`--nocapture` prints the table); real loop with `--cf02 --autopilot --cinematic-finishes` | **0 moments in every attempt of every room fixture** (table below); real loop 0 `CINE01_MOMENT` lines over the 487-tick clear. Positive control in the real loop: `--fixture cp03_goblin_pileup --cinematic-finishes` emits `CINE01_MOMENT tick=139 tier=ordinary score=7` and a completed slow motion |
| Frame time | none | **NOT RUN** on real hardware (see the handoff) |

### Q41: cinematic count per arena attempt

The CINE-01 evaluator runs whatever the presentation setting is, so these counts equal the count with the cinematic on. They are fed the way the live loop feeds them (`push_driven`, one attempt per `Restart`). Every cell is the number of emitted moments in that attempt.

| Room fixture | Attempts | Moments per attempt |
| --- | --- | --- |
| `cf02_arena_direct` (solo, direct plan, assistant on Donut) | 1 | 0 |
| `cf02_arena_environment` (solo, shore plan) | 3 (the plan dies twice, U4) | 0, 0, 0 |
| `cf02_arena_swap` | 1 | 0 |
| `cf02_arena_coop` (2P) | 1 | 0 |
| `cf02_arena_three_attempts` | 4 (three defeats) | 0, 0, 0, 0 |
| `cf02_goblin_self_own` (the goblin trips the shore) | 1 | 0 (a `ShoreTripped` never scores, Q41) |
| `cf02_block_hurts_carl`, `cf02_block_hurts_partner`, `cf02_block_spares_left_striker` | 1 each | 0 |
| `cf02_intact_shore_ignores_goblins`, `cf02_parapet_cover`, `cf02_perch_slab_climb`, `cf02_perch_above_spit`, `cf02_perch_dive` | 1 each | 0 |

Reading: the scripted plans never produce the skill-gated chain CINE-01 asks for (a clean melee plus block chain or a mixed chain with a proven living contact), so slow motion stays rare, as Q25 and Q41 require. A human who kicks goblins into the wall and breaks the shore in one chain may still earn a moment; whether that feels right is an owner question.

### Fixture hashes (80 of 80 match the committed replays)

`hash` is the `PARTY_STATE` hash of a fresh headless run, equal to the committed value in the named folder.

| Fixture | hash | replay folder under `docs/evidence/` | result |
| --- | --- | --- | --- |
| `cf02_llama_swat_interrupt` | `bd692a6b647a2a41` | cf02/replays | match |
| `cf02_llama_swat_guard` | `764dbd2f40240d1e` | cf02/replays | match |
| `cf02_llama_pounce` | `b463444627b7960e` | cf02/replays | match |
| `cf02_llama_bolt_stops` | `579f344f68263c67` | cf02/replays | match |
| `cf02_donut_solo_llama` | `e1eae0ffae9bc431` | cf02/replays | match |
| `cf02_donut_bailout` | `50c5db024da0ce67` | cf02/replays | match |
| `cf02_swap_eased_pan` | `cb19d498fcbcaee8` | cf02/replays | match |
| `cf02_assist_swat_interrupt` | `8d3ecbb947c32984` | companion-ai | match |
| `cf02_assist_swat_trailing_gap` | `04841f57c5c450c5` | companion-ai | match |
| `cf02_assist_pounce_staggered` | `13fe359d33b2ee65` | companion-ai | match |
| `cf02_assist_hp_drops_midmove` | `59fd86eb22c57e86` | companion-ai | match |
| `cf02_assist_cadence` | `b6639df01f0b99a9` | companion-ai | match |
| `cf02_assist_magic_locked` | `3fa0c4ae6aaecf92` | companion-ai | match |
| `cf02_assist_missile_reserve` | `fa241a206ba24e8e` | companion-ai | match |
| `cf02_assist_camera_parity` | `e3b30a855b7fe5b0` | companion-ai | match |
| `cf02_kick_then_stomp` | `6a38f39b5110dc90` | cf02/replays | match |
| `cf02_stomp_whiff` | `fc5181f6b763f50b` | cf02/replays | match |
| `cf02_stomp_standing` | `46b56f9470306d5e` | cf02/replays | match |
| `cf02_stomp_no_stand_up` | `31a7c769af4e3a5e` | cf02/replays | match |
| `cf02_confirm_kick` | `838403b03715e566` | cf02/replays | match |
| `cf02_whiff_no_confirm` | `ab76e42fd4550ea2` | cf02/replays | match |
| `cf02_down_physical_beats_confirm` | `439c2ea3a5c7f26a` | cf02/replays | match |
| `cf02_jab_over_lying` | `040f2cfb342eea76` | cf02/replays | match |
| `cf02_lethal_kick_far` | `13f5c26a5a91056c` | cf02/replays | match |
| `cf02_pounce_keeps_down` | `b7bd1e7af250a940` | cf02/replays | match |
| `cf02_pounce_hold` | `ba3f13faa9659baa` | cf02/replays | match |
| `cf02_stomp_boss` | `480fc8ad59b022fe` | cf02/replays | match |
| `cf02_2p_same_tick` | `da1a168a6bc17a3e` | cf02/replays | match |
| `cf02_air_kick_jump_in` | `ee11c7830e16f82b` | cf02/replays | match |
| `cf02_air_kick_relaunch_once` | `0cf54de7d6c3c82e` | cf02/replays | match |
| `cf02_dive_lying` | `fbb344ba67d525a8` | cf02/replays | match |
| `cf02_dive_standing_knockdown` | `8b4ecb9793970774` | cf02/replays | match |
| `cf02_dive_bounce_kick` | `c35873ca28d4ae31` | cf02/replays | match |
| `cf02_dive_whiff_recovery` | `f6e6165ea7f2b805` | cf02/replays | match |
| `cf02_bailout_escapes_swing` | `ac675d2f15e777af` | cf02/replays | match |
| `cf02_bailout_no_attack_until_land` | `2d5c027bbe128174` | cf02/replays | match |
| `cf02_buffer_turns` | `1a5f85cefbd120ef` | cf02/replays | match |
| `cf02_buffer_no_early_chain` | `027414d51818e9e7` | cf02/replays | match |
| `cf02_jump_attack_same_tick` | `d0ecfb8433d01685` | cf02/replays | match |
| `cf02_landing_grace` | `75ce9b552aea4b3c` | cf02/replays | match |
| `cf02_cancel_table` | `7721fbd0b323063f` | cf02/replays | match |
| `cf02_dive_boss` | `8ec797371160639d` | cf02/replays | match |
| `cf02_contact_knockdown` | `ad3e4dbdf76df04c` | cf02/replays | match |
| `cf02_contact_below_threshold` | `cb311ad6d6bbf3cc` | cf02/replays | match |
| `cf02_contact_cancels_swing` | `2a19dd6ac146207b` | cf02/replays | match |
| `cf02_no_chain` | `81bb784964470183` | cf02/replays | match |
| `cf02_body_bowl` | `bdfd16d8979ebe75` | cf02/replays | match |
| `cf02_air_relaunch_contact` | `58c789f182f2ea10` | cf02/replays | match |
| `cf02_wall_slam_kill` | `e887c36e54d1848e` | cf02/replays | match |
| `cf02_wall_slam_body` | `0e6f84ebfe3f2e09` | cf02/replays | match |
| `cf02_layout_edge_no_slam` | `fc74bfc67a33e54b` | cf02/replays | match |
| `cf02_morale_witness_hesitate` | `9bea8f387364f45e` | cf02/replays | match |
| `cf02_morale_spectacle_panic` | `2a24ee538108ee29` | cf02/replays | match |
| `cf02_morale_panic_once` | `7b6c2b4c7a3194b0` | cf02/replays | match |
| `cf02_morale_last_goblin` | `1aaf8db3ff0b63d1` | cf02/replays | match |
| `cf02_morale_cornered` | `9cae17f3befaa95f` | cf02/replays | match |
| `cf02_morale_enraged_tell_unchanged` | `897bdf0bdb9921bd` | cf02/replays | match |
| `cf02_coward_backoff_kick` | `fc6afbe4fc6a2ab4` | cf02/replays | match |
| `cf02_coward_jab_still_hits` | `5fa5b79dab68a5f7` | cf02/replays | match |
| `cf02_showoff_taunt_punish` | `46183086d8be3179` | cf02/replays | match |
| `cf02_pincer_token` | `811d0b6d7b322c35` | cf02/replays | match |
| `cf02_restart_reshuffles` | `4962783a758ca230` | cf02/replays | match |
| `cf02_arena_direct` | `f2f36d36993a9288` | cf02/replays | match |
| `cf02_arena_environment` | `5146be1636184a36` | cf02/replays | match |
| `cf02_arena_swap` | `cd98baad98a101ee` | cf02/replays | match |
| `cf02_arena_coop` | `5e46d9f55d160c60` | cf02/replays | match |
| `cf02_arena_three_attempts` | `f197596551d87996` | cf02/replays | match |
| `cf02_block_hurts_carl` | `f7ab5b9fc93a510f` | cf02/replays | match |
| `cf02_block_spares_left_striker` | `32e12c30d3466045` | cf02/replays | match |
| `cf02_block_hurts_partner` | `287bfca0058d730d` | cf02/replays | match |
| `cf02_goblin_self_own` | `4fe0d16b10e0ff75` | cf02/replays | match |
| `cf02_intact_shore_ignores_goblins` | `3b8dadb572c271e7` | cf02/replays | match |
| `cf02_parapet_cover` | `69bfb415a9724d06` | cf02/replays | match |
| `cf02_perch_slab_climb` | `2e1dfe90f447b9c2` | cf02/replays | match |
| `cf02_perch_above_spit` | `18ffe87e2a39e33d` | cf02/replays | match |
| `cf02_perch_dive` | `e74bd1fe4fcd0ebf` | cf02/replays | match |
| `companion_carl_interrupts_for_donut` | `932624152d49df54` | companion-ai | match |
| `companion_donut_supports_carl` | `2d03dbced1ed5647` | companion-ai | match |
| `companion_duo_no_ai` | `7bbaea9a82c73afe` | companion-ai | match |
| `companion_swap_mid_move` | `ec01b32eeb394f8f` | companion-ai | match |

### Golden inventory

No existing golden moved on the way from the CF02-00 baseline (`a572b89f`) to this base. The only changed pinned literal in `src` outside the new CF02 fixtures is the intended `cf02::TUNING_HASH` (new knobs). The tests that pin the legacy goldens all pass in the 1716: the 20 `EXPECTED_HASHES` (`combat_scene::tests::fixtures_match_the_committed_expected_hashes`), `GOLDEN_WALK_RIGHT_400` (`corridor::tests::replay_state_hash_is_golden_and_repeatable`), the 7 party goldens, the 13 boss goldens, the 8 `q10_*` Donut goldens, the `demo01_*` probes (the demo CI row), the CINE goldens, the companion and autoplay pins (`autoplay::tests::pinned_initial_hashes_match_a_fresh_sim`, `companion_fixtures` goldens) and `cf02_parity_off`. In `docs/evidence`, no committed replay of another work order changed: the diff against the baseline only adds files (the `gob-01` and `gob-02-death` art evidence were regenerated by their own art PR, #155, and are not simulation goldens).

## Defects found and fixed in this PR

1. **`COMPANION_CONFIG` said `inactive` for every `--cf02` and `--demo` run.** `--demo` fills in a fixture, and the label treated any fixture as scripted, so a live human `--cf02` run printed `COMPANION_CONFIG: set=cf02_assist inactive` although the assistant does run (the live party is chosen by `manual`). Label only: no simulation, hash or input changed. Fix: `Args::companion_inactive` (a fixture counts as scripted only when the run is not `manual`), one unit test (`the_companion_config_line_is_active_only_in_a_live_human_run`). Located by the live idle smoke above.
2. **U2 (from the #158 review): the CI probe "`--cf02` differs from `--demo`" proved nothing about the rules.** `--cf02` loads its own room, so the two replays differ whatever the rules do. `scripts/ci_floor01_combat.sh headless` now requires the `--cf02` whole-route autopilot replay to log a rules-only event (`goblin_reaction`, goblin morale), the `--demo` replay to log none, and `--cf02 --cf02-seed 7` to replay differently from the default seed (`CF02_FLAG_LIVE_OK: ...`). **What it guards, exactly:** that `--cf02` loads the arena and the arena runs the CF02 rules and morale, that `--demo` stays free of them, and that `--cf02-seed` reaches the temperament draw. **What it does not guard:** the `CombatState::with_cf02()` call in `floor01_scene.rs` (the review removed it and the replay stayed byte-identical, because the arena fixture names already switch the rules on, so the call is redundant on the arena path: follow-up FU-D). The script ran green after the change (173 `*_OK` lines).

## Adopted defaults of CF02-REVIEW

- **CF02-REVIEW-A01:** the Q41 measurement is a committed, test-only harness (`cf02_room_cinematic_count_per_attempt`, no `src` behaviour) so the number is reproducible; it asserts at most one moment per attempt and none for the self-own, the three block fixtures and the defeat run.
- **CF02-REVIEW-A02:** the local `needless_range_loop` findings are recorded, not fixed, because they sit in files owned by merged stages and CI on stable is the gate; a follow-up may tidy them.
- **CF02-REVIEW-A03:** the scorecard pre-fills the build SHA with the tested main base above; the owner records `git rev-parse HEAD` of the build the owner actually plays (the playtrace manifest also records the commit and a source fingerprint).

## Reproduce

```sh
cargo build --locked --bin floor01_scene
bash scripts/export_assets.sh && python3 scripts/validate_assets.py
bash scripts/ci_floor01_combat.sh headless && bash scripts/ci_floor01_combat.sh demo
./target/debug/floor01_scene --list-fixtures cf02_      # 76 names
./target/debug/floor01_scene --list-fixtures companion_ # 4 names
cargo test --locked --lib cf02_room_cinematic_count -- --nocapture
xvfb-run -a ./target/debug/floor01_scene --cf02 --autopilot --scale 1 --no-audio --exit-after-ticks 1800
```

The CF02-00 audit tables are deterministic (`python3 scripts/cf02_baseline_audit.py docs/evidence/cf02/audit`); the per-stage captures and GIFs are software-renderer frames at fixed ticks, evidence of what was drawn, not reproduced bit for bit by CI.

Rights: every frame shows only the committed original Carl, Donut, goblin and llama art under the recorded scope in the [rights register](../../04-production/rights-and-release.md). This PR adds no art, audio or image. Model attribution: Claude Sonnet executed CF02-REVIEW; the independent top-tier final review follows the PR.
