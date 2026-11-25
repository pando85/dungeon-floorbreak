# CORRAL-F: room 0 (the Infernal Corral) listed, with its evidence

**Status: merged, game-feel review pending** once the PR merges. Nothing here is accepted: the owner judges the brace read, the rush, the stationary-spam margin and the room's feel by playing it (OQ1 to OQ5 below stay open).

CORRAL-F lists room 0 as the **first room** of `assets/levels/f01_late/manifest.json` at milestone `greybox` (the `corral` row moved from `unlisted_rooms` to `rooms[0]`; `unlisted_rooms` is now empty). A bare `--f01-run` starts in the corral, and its exit relaunches into the Hoarder. Out of scope and not run here: CORRAL-D (the partition), LLAMA-03 (rush art), AREA-HEAT-01.

Every run below is a scripted pilot or a fixture, never a human. Window runs are xvfb runs of the debug binary at `--scale 1` (the PLAYTRACE recordings with `--no-audio`; the captures with `scripts/room_capture.sh`'s own flags).

## Prerequisites from the #280 review

| Item | Result | Evidence |
| --- | --- | --- |
| (a) The brace reads at 1x | Stand-in changed, readability **open (OQ1)** | `combat_anim::rush_stand_in`: the brace lean is 4 px (was 2), rocks 1 px forward between scrapes, and every scrape puff is 3x3 (was 2x2); test `the_brace_reads_at_1x_as_a_pumping_lean_with_3x3_puffs`. [`rush-brace-1x.png`](rush-brace-1x.png) is 1x crops (200x90 each); [`rush-brace-x3.png`](rush-brace-x3.png) is the same sheet enlarged 3x (nearest). Rows: the `corral_rush_bonk_wall` fixture (ticks 18, 40, 52), `corral_rush_hits_idle_carl` (18, 40, 60), and the in-room rush (746, 782, 792). The puffs show but stay small. Whether the brace reads as a rush and not a spit at 1x (hard review gate (c)) is the owner's call. |
| (b) A corral room pilot that plays M0 to M3 | PASS | `--room-pilot corral` (`corral:WHO:SEED`, WHO `carl_solo`, `donut_fights`, `companion`); `room_pilots::tests::the_corral_room_pilot_replays_the_library_run_at_every_gate_seed` (party hash equal to the library run's). `--room-pilot corral_rush` is an evidence plan: Carl clears M0 and M1, parks inside B's trigger and is rushed (`the_corral_rush_plan_parks_carl_and_b_rushes_him_in_the_room`). |
| (c) Native 640x360 and 640x400 captures of the M3 grate and an in-room rush, window hash equal to headless hash | PASS | [`captures.txt`](captures.txt): four `match=yes` lines plus the M0 missile capture. Stills below. |

### Stills (native, unscaled)

| File | What it shows |
| --- | --- |
| [`rush-640x360-tick746-brace.png`](rush-640x360-tick746-brace.png), [`rush-640x400-tick746-brace.png`](rush-640x400-tick746-brace.png) | B's brace, tell tick 4 (tell on 742) |
| [`rush-640x360-tick782-charge.png`](rush-640x360-tick782-charge.png), [`rush-640x400-tick782-charge.png`](rush-640x400-tick782-charge.png) | the committed charge (commit on 774, 32 ticks after the tell) |
| [`rush-640x360-tick792-contact.png`](rush-640x360-tick792-contact.png), [`rush-640x400-tick792-contact.png`](rush-640x400-tick792-contact.png) | the tick after contact (contact on 791, Carl hurt, health 1 left) |
| [`gate-640x360-tick859-closed.png`](gate-640x360-tick859-closed.png), [`gate-640x400-tick859-closed.png`](gate-640x400-tick859-closed.png) | the M3 grate standing (greybox, code-drawn) while the dock llama lives |
| [`gate-640x360-tick867-open.png`](gate-640x360-tick867-open.png), [`gate-640x400-tick867-open.png`](gate-640x400-tick867-open.png) | the grate gone after the second M2 defeat (tick 863) |
| [`m0-640x360-tick44-first-missile.png`](m0-640x360-tick44-first-missile.png) | Donut's first missile in flight toward M0's first goblin (hit on tick 48) |

The 640x360 and 640x400 hashes differ from each other because the view height is part of the simulation (what is on screen); each size's window hash equals its own headless hash.

## MAG-01 to MAG-16 (`docs/01-design/floor-01-llama-magic-baptism.md` section 8)

| Test | Result | Evidence and reason |
| --- | --- | --- |
| MAG-01 | PASS (room 0 part) | `f01_corral_fixtures::tests::corral_room0_no_grant`: every pilot enters with mana 8/8 (the Guild biscuit read through) and room 0 grants nothing; `f01_corral::tests::the_story_loadout_holds_nothing_in_room_0` (Carl holds no special); both PLAYTRACE sessions log 0 `CapabilityGranted`. The Guild transition itself is not part of the floor run (the run starts at room 0), so the unlock-exactly-once moment is not re-run here. |
| MAG-02 | PASS | [`journal_m0_m1.txt`](journal_m0_m1.txt): `MagicMissile` started on tick 31, mana spent on 35 (cast tick 4, the release), hit goblin 0 for 1 damage on 48; Carl's jab finishes goblin 0 on 59 (the melee alternative). Still: `m0-640x360-tick44-first-missile.png`. Mechanism tests: `combat_scene::donut` `a_point_blank_enemy_is_hit_on_the_release_tick`, `the_bolt_spawns_ahead_of_donut_flies_6_px_a_tick_and_fizzles_at_240_px`. |
| MAG-03 | PASS | `corral_eval::tests::corral_auto_m1_static_control_on_the_room` (three honest bolts, 0 HP lost, no rush tell) and `corral_auto_m1_positive_control_a_stationary_donut_wins_m1`; `corral_fixtures::tests::the_m1_llama_has_party_targeting_but_never_rushes`. Journal: the M1 llama tells on 139, spits on 175 (the glob after its warning), is defeated on 301. |
| MAG-04 | PASS (mechanics); 1x read open (OQ1) | Rush tell 32 ticks (`enemy_llama_rush_v0.json` `crlq06_tell_ticks`), the spit's 36: in the room, tell 742, commit 774, contact 791 ([`captures.txt`](captures.txt)). Tests: `the_trigger_edge_tells_at_128_and_walks_first_at_132`, `rush_hits_idle_donut_with_the_full_timeline`, `the_committed_rush_never_turns_toward_a_hero_who_lands_behind`, `a_rush_never_spits_and_telling_is_false_from_attack_tick_32`. |
| MAG-05 | PASS | `running_away_clears_from_attack_tick_44_and_is_hit_from_45`, `a_jump_clears_donut_on_attack_ticks_4_to_43_and_not_3_or_44`, `a_jump_clears_carl_on_attack_ticks_12_to_42_and_not_11_or_43`, `two_heroes_in_the_lane_only_the_nearer_is_hit_once`, `point_blank_contact_is_active_tick_one`, `a_rush_stops_at_the_leash_bound_with_a_miss`, `the_recover_runs_40_ticks_then_the_cooldowns_are_set`. |
| MAG-06 | PASS | `a_bolt_pressed_on_20_or_21_interrupts_the_tell_and_22_lands_in_the_commit`, `an_interrupted_tell_raises_both_cooldowns_to_30`, `a_lethal_hit_in_the_commit_downs_the_llama_in_place_with_no_contact`. In the recorded sessions the dock llama's spit tell is interrupted (`LlamaInterrupted` on 801) and the room still clears. |
| MAG-07 | PASS | `corral_eval::tests::corral_auto_m2_heights_real_globs_and_rushes`, `a_floor_donut_is_never_spat_at_the_llama_walks_into_the_128_trigger_and_rushes`, `a_hopping_floor_donut_is_never_spat_at_and_a_tell_starts_only_on_a_grounded_tick`, `carl_on_the_floor_is_spat_at_and_not_rushed_beyond_the_trigger`. |
| MAG-08 | PASS | `a_lane_end_outside_the_view_starts_no_tell`, `a_dormant_llama_holds_until_a_living_hero_is_within_320_then_wakes_and_rushes`, `the_solo_bias_makes_the_llama_target_the_controlled_donut`; `corral_route_m0_m3_every_pilot_reaches_the_exit` checks that both M2 llamas wake and attack. |
| MAG-09 | NOT RUN | CORRAL-D (the partition) is out of scope; the CORRAL-AUTO partition variants stay `not_run`. |
| MAG-10 | PASS | `corral_auto_press_edges_and_mana_accounting_close_on_every_run`; the report's "Mana accounting closes on every run" is `pass`. This PR changes no mana or cast value. |
| MAG-11 | PASS (gate); margin open (OQ3) | The CORRAL-AUTO corral gate: KC19 decisive variant `pass` (2122 groups judged, 0 failing), CRL-Q35 invariant `pass`, KC19 with the reflex off `pass`. Diagnostic, not gated: 3204 of 12732 gated parked M2 runs defeat the llama of their own level (`layer_static_clear`, OQ3). |
| MAG-12 | PASS (scripted); 2-pad run NOT RUN | `corral_route_m0_m3_every_pilot_reaches_the_exit` (solo Carl, solo Donut, two players, seeds 0..3) and the gate probe below (with the `cf02_free` companion); the report's mobile Donut, Carl melee and 2P co-op victories on M2 are `pass`. A real two-pad session is NOT RUN. |
| MAG-13 | PASS (rush and spit); breakage NOT RUN | `corral_fixtures::room_tests::a_retry_in_m2_restores_both_llamas_from_the_fixture`, `a_retry_during_a_commit_restores_the_rush_state_the_seq_and_the_cooldowns`. Breakage needs CORRAL-D. |
| MAG-14 | PASS (scripted); human attempts NOT RUN | Two PLAYTRACE recordings of the room pilot, each verified strict twice in fresh processes ([`playtrace_sessions.txt`](playtrace_sessions.txt)); `corral_auto_double_run_equality`; the native captures above. The recordings are a scripted bot, not real player attempts. |
| MAG-15 | NOT RUN | Needs three human play styles; the owner's playtest. |
| MAG-16 | PASS (evasion, audio off, HUD reduced motion); a reduced-VFX mode does not exist | New `f01_corral_fixtures::tests::corral_rush_does_not_consume_reflex`: on the four route pilots (seed 0) no evade is logged (one rush contact, 0 evades); with Donut parked inside B's trigger and her reflex armed, B's rush hits her on its contact tick, no `HeroEvaded` is logged, and the hurt disarms the reflex. Also `corral_fixtures::tests::an_armed_reflex_never_evades_the_rush_and_the_hit_resets_it` and `corral_auto_the_reflex_variants_are_real`. The rush plan captured again with `--no-audio --hud-reduced-motion` keeps the same window and headless hash (`aabf4ee965f78447`, [`captures.txt`](captures.txt)); 2 of its 30 frames differ in pixels (presentation only) and the brace frame is identical. The game has no reduced-VFX setting, so that half is not run. |

**Hard review gates (a) to (f):** (a) KC19 passes on the gate, margin in OQ3; (b) the corral llamas target the party (`the_solo_bias_makes_the_llama_target_the_controlled_donut`); (c) **open, OQ1**; (d) Donut has full mana and casts in M0 (MAG-01, MAG-02); (e) the report's Carl melee victory on M2 is `pass`; (f) `a_lane_end_outside_the_view_starts_no_tell`. None of these is an approval: the owner reviews.

## CORRAL-AUTO

`cargo test --locked corral_auto`: 17 passed. `corral_eval --out DIR` on this head writes a `report.md` and a `summary.json` byte-identical to [`../corral-c/report.md`](../corral-c/report.md) (sha256 `09edc05333079f270161f2449f4e2c7849045a07c6e60cf364b1c52bda2e7bdb`) and [`../corral-c/summary.json`](../corral-c/summary.json) (sha256 `466011c1ede792d6df59650b633514f5442d2d1fad4d0d891ee2c9f2aee41d46`); the files are not copied again. The baseline rows that fail in that report are the corridor "before", not the corral.

## Gate probe (`f01_corral_fixtures::run`, story loadout)

| Pilot | M3 gate open (= second M2 defeat) | Exit reached | Longest stuck run (Carl, Donut) | Deaths |
| --- | --- | --- | --- | --- |
| `carl_solo` | 863 | 971 | 48, 0 | 0 |
| `donut_solo` | 862 | 973 | 1, 28 | 0 |
| `companion` | 1017 | 1125 | 48, 0 | 0 |

Seeds 0 to 8 give these same ticks and stuck values for every pilot: the seed (the goblins' temperaments) does not change these runs' outcome. The seed still changes the state hash of a fixture-input run (`the_room_replays_to_the_same_hash_twice_and_the_seed_matters`).

## Listing and the floor run

- The manifest's `rooms` are now `corral, hoarder, boiler, hazards, ring_court, ring_split, juicer, mastery`; `Manifest::validate` requires a listed corral to be the first room and a corral scene to be named `corral`.
- `f01_run`: `first_room` is the corral and `next_room(corral)` is the Hoarder. The corral has no story runtime, so `carry_for` hands the Hoarder an empty story ledger (CF-AD3): the panel reads `ROOM 1/8 CLEARED: THE INFERNAL CORRAL` and `The Hoarder (story ledger carried)`, and a carried Hoarder launch has the same story snapshot and state hash as an uncarried one (`the_corral_exit_relaunches_into_the_hoarder_with_its_ledger`, and the binary test `the_corral_resolves_through_the_late_launcher_listed_first`).
- `scripts/ci_floor01_combat.sh` checks `listed=true` and that `--f01-run` prints `F01_CORRAL_ENTRY`; `scripts/check_f01_owner_commands.sh` launches the corral as a listed room (`ok`).

### Moved pins

| Pin | From | To | Reason |
| --- | --- | --- | --- |
| `f01_late::MANIFEST_HASH` (`f01_late_manifest`) | `0x727f_c5cf_5fd6_2109` | `0xbb11_225e_c03d_7e0b` | the `corral` row moved from `unlisted_rooms` to the first entry of `rooms` (and its `status` text) |

No committed recording stamps the old manifest pin (searched in hex and decimal). No other data pin, golden or replay line moved.

## Adopted defaults (recorded, not owner decisions)

- **CF-AD3:** an inert story room (the corral) carries the empty story ledger to the next room, so the Hoarder starts from her own pin and the panel says the ledger was carried, instead of "TEST LOADOUT, nothing carried".
- **CF-AD4:** the corral gets no render replay lines in `scripts/make_f01_gate_d_replay.sh` (that file's count of 153 lines is pinned by its own check); the window-equals-headless captures above stand in for them.
- **CF-AD5:** the seed sweep of the gate probe is reported as-is (seeds 0 to 8 identical) rather than adding a seed-sensitive pilot.

## Open owner questions (not decided here)

- **OQ1, the brace read:** does the 4 px pumping brace with 3x3 puffs read as a rush (not a spit) at 1x? Real rush art is LLAMA-03.
- **OQ2, evasion:** the rush is never evadable by Donut's reflex (DM-Q18, CRL-Q18). Is that the wanted feel in room 0?
- **OQ3, per-level stunlock:** 3204 of 12732 gated parked M2 runs defeat the llama of their own level (`layer_static_clear`); is that margin acceptable?
- **OQ4, the exit gate:** the M3 grate opens only on the second M2 defeat (CRL-Q26) and is a code-drawn greybox; keep it, or open the exit earlier?
- **OQ5, awareness notes:** the bumper-top stall spot (`corral_auto_the_rush_runs_and_the_bumper_top_is_a_stall_spot`), the entry-shelf stall, DV-2 (one optional entry shelf, the other shelf became the exit dock), DV-10 (the partition stands across the pen's only floor lane, CORRAL-D) and CRL-Q42 (jumping in place counts as stationary for the gate).

## NOT RUN

MAG-09 and the breakage half of MAG-13 (CORRAL-D); MAG-15 (human styles); a real two-pad session; a reduced-VFX run (no such setting exists); a human-played floor run starting in the corral; a release build; real hardware.

## Files

[`SHA256SUMS`](SHA256SUMS) lists every file of this directory except itself and this README.
