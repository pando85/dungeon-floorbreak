# DONUT-PROG-01 DP-A evidence: ability rows, flags and unlock triggers

Headless replays of the `donut_prog_*` fixtures (`replays/headless_<fixture>.txt`, the
`COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of
`floor01_scene --headless-floor01-smoke --party-fixture <name>`). Each replays twice
identically, equals its committed file (unit test and the `headless` loop of
`scripts/ci_floor01_combat.sh`) and has its golden pinned in `src/donut_prog_fixtures.rs`.
Placeholder status: the real safe-room (I-08) reward pickup and the Hoarder content are
later work, so the fixtures use the placeholder zone `f01_safe_room_reward_found` in the
corridor run and the boss arena. DP-A has no presentation and no simulation rule: it adds
rows, flags, triggers and data, so a render-loop replay is added by the first DONUT-PROG PR
that draws something (DP-D).

Hashes and ticks are those of the agile Donut body (DM-A, #186), which is the default motion since this branch was rebased onto it; the first recording of these fixtures (legacy body) differed only in Donut-driven ticks and hashes.

| Fixture | Final `PARTY_STATE` hash | What it shows |
| --- | --- | --- |
| `donut_prog_story_unlocks` | `f7eb55392e6c5a98` | the biscuit at the guild zone (tick 50), then one `capability_granted` (tick 246, hero 1, `jump_reach_boost`) at the safe-room placeholder zone; Circle/B presses emit nothing |
| `donut_prog_grant_survives_retry` | `b3e9f4b4c2f6eff8` | the grant at tick 246, a defeat at tick 440 and a party retry at tick 489 (twice): the capability stays held and is never granted a second time |
| `donut_prog_primary_mapping` | `cd026290cfd5d561` | Square/X swats before the biscuit, casts eight missiles after it and is dry at 0 mana; Circle/B pressed four times does nothing |
| `donut_prog_boss_pins` | `0a03af025ce95b25` | under Donut's boss pin (the boost, the biscuit read through) the Juicer defeat (tick 311) grants only `special_short_blink` (hero 1) on that tick |
| `donut_prog_boss_both_programs` | `befb7d69bbe94cfd` | with both programs the defeat tick grants `fist_reinforced` (hero 0) and then `special_short_blink` (hero 1), in grants-list order |

Event log of both Juicer grants (`donut_prog_boss_both_programs`):

```
COMBAT_EVENT tick=311 kind=enemy_defeated target=16981 cause=drop_hazard credit_player=0 rack=0
COMBAT_EVENT tick=311 kind=capability_granted hero=0 capability=fist_reinforced
COMBAT_EVENT tick=311 kind=capability_granted hero=1 capability=special_short_blink
```

Event log of a grant and two defeat retries (`donut_prog_grant_survives_retry`):

```
COMBAT_EVENT tick=50 kind=unlock_granted hero=1 unlock=primary_ranged
COMBAT_EVENT tick=246 kind=capability_granted hero=1 capability=jump_reach_boost
COMBAT_EVENT tick=440 kind=hero_defeated hero=donut
COMBAT_EVENT tick=489 kind=restart deaths=1
COMBAT_EVENT tick=440 kind=hero_defeated hero=donut
COMBAT_EVENT tick=489 kind=restart deaths=2
```

The checkpoint respawn and the fixture restart are unit tests in `src/donut_prog_fixtures.rs`
(`a_checkpoint_respawn_keeps_a_donut_grant_and_a_fixture_restart_removes_it`,
`donut_prog_grant_survives_retry`). Every earlier fixture, golden and replay is unchanged,
the `carl_prog_*` replays included.

## DP-D: the presentation of the blink (HUD, cues, drawn camera)

DP-D draws what DP-C simulates; it changes no outcome, so every earlier fixture, golden and
replay is unchanged. The four new render fixtures below are in the `donut_prog_*` registry (so
the CI `headless` loop replays each twice and diffs it against `replays/`); the three that run in
the microarena or on the corridor are scripted P2-Donut runs, `donut_prog_grants_2p` uses an
evidence-only table variant (see its doc comment: the shipped table grants the blink on the
Juicer's defeat, which the render loop cannot run because the boss pilot is a headless script).

| Fixture | Final `PARTY_STATE` hash | What it shows |
| --- | --- | --- |
| `donut_prog_grants_2p` | `ab6921d3eba4608d` | tick 246: `jump_reach_boost` then `special_short_blink` (hero 1) on one tick; the notices `LIGHT FEET` (246) and `PUDDLE JUMPER` (412, after 136 + 30 ticks) queue; her socket turns from the padlock to the equipped chip on the grant tick |
| `donut_prog_blink_fizzle` | `6e52421a4f95bf63` | `special_started` 136, `blink_fizzled reason=no_destination` 142: the destination was valid at the press, then Carl ran on and the scroll camera moved the 24 px inset past it; nothing spent, no cooldown, one shrinking ring |
| `donut_prog_arena_blink_tour` | `90775214f5ef5cbe` | flat blink 340 -> 452 (press 40, arrival 46); `special_ready` 586 (arrival + 540); the second blink from 656 stops at 724, in front of the 32 px parapet at 736 (arrival 606) |
| `donut_prog_arena_blink_edge` | `6ae2cb2ece513533` | `special_rejected reason=no_mana` 270 (pool cast down to one charge) and `reason=no_destination` 420 (no destination inside the view): each flashes the socket |

Real windowed Bevy captures at 640 x 360 (1x), from `scripts/make_dp_d_evidence.sh` (needs a debug
build, `scripts/export_assets.sh`, xvfb, ffmpeg and ImageMagick; the renders are driver dependent and
are not reproduced bit for bit by CI; the deterministic half is the replays and the tests). Checksums
in `SHA256SUMS`.

| File | Shows |
| --- | --- |
| `dp_d_gif1_grants.gif` (1x, ticks 240..534, every 6th) | the two grant notices queued at the bottom of the screen and the socket turning equipped |
| `dp_d_gif2_cycle_1x.gif` (1x, ticks 6..720, every 6th) | the flat blink, the socket muted with its underline shrinking over 9 s, the ready flash, the wall blink |
| `dp_d_gif3_flat_3x.gif` (3x crop, ticks 36..55) | crouch, ripple at her feet, the dotted streak, the destination ripple |
| `dp_d_gif4_wall_3x.gif` (3x crop, ticks 596..615) | the blink stopping in front of the parapet |
| `dp_d_gif5_fizzle_1x.gif` (1x, ticks 132..155) | the arrival fizzle and the single shrinking ring |
| `dp_d_gif6_refusals_3x.gif` (3x of her plate) | the no-mana (266..279) and no-destination (416..429) refusals and the socket flash |
| `dp_d_cues_sheet.png` (3x, ticks 39..49) | the blink one tick at a time |

The HUD of a captured sequence is now stepped once per simulated tick (a presentation-only fix in
`capture_sequence`): before it, a capture kept the HUD of its first frame, so the socket, its flash
and the notices could not show.

Measured for the PR: `donut_prog_blink_drawn_view_human` (human-controlled Donut, 216 runs: 640, 768 and
960 rest widths, 16:9 and 16:10, both blink directions, with and without a far threat) found **0**
ticks outside the drawn view after any arrival (grace 12). The existing `something_outside` path
already keeps a framing subject's safety region in view, so `scene_camera.rs` needed no change.

## DP-F: pilots, the two deferred captures and the handoff

Last DONUT-PROG stage. It moves no golden and no earlier replay; the `donut_prog_*` registry grows from nine to twelve fixtures (three render fixtures with committed replays below), and the owner handoff and scorecard are in [HANDOFF.md](HANDOFF.md).

### Render fixtures and captures (the DP-B and DP-E deferrals)

| Fixture | Final `PARTY_STATE` hash | What it shows |
| --- | --- | --- |
| `donut_prog_arena_ledge_base` | `970a169e812a4bd8` | the microarena with a 128 px ledge; P2 Donut (biscuit, no boost) runs into it and jumps on tick 40: she peaks under the top and falls back to the floor |
| `donut_prog_arena_ledge_boost` | `98b7c723c72f71bc` | the same script and seed with `jump_reach_boost` (`--donut-light-feet`): she lands on the ledge (grounded on it at tick 80) |
| `donut_prog_arena_blink_cornered` | `a375733d805c1725` | the AI Donut (jump dodge radius 0, target bias 0, exactly the 2 charge reserve) with a goblin 28 px away: `goblin_tell` 5, `special_started` 20, `mana_spent` (left 0) and `blink_resolved` 88 to 180 on 26, `goblin_swing` 31 finds no one |

Real windowed Bevy captures at 640 x 360 (1x source), from `scripts/make_dp_f_evidence.sh` (debug build, `generated/art`, xvfb, ffmpeg, ImageMagick; driver dependent, not reproduced bit for bit by CI). Checksums in `SHA256SUMS`.

| File | Shows |
| --- | --- |
| `dp_f_gif1_ledge_128_side_by_side.gif` (2x crops, ticks 30..100, every 2nd) | left: no boost, right: `jump_reach_boost`; same ledge, same script |
| `dp_f_gif2_ai_escape_3x.gif` (3x crop, ticks 2..47) | the AI Donut blinking out of the goblin's swing |
| `dp_f_escape_sheet.png` (2x, ticks 18..30) | the crouch, ripple, destination ripple and dotted streak around the press (20) and the arrival (26) |

### Encounter-completion pilots (DP-Q47, harness tests)

`pilot_reports.txt` is the per-run report (`DP_F_PILOT_OUT=<file> cargo test --locked --lib print_the_pilot_reports`). Totals:

| Group | Runs | Cleared | Deaths | Blinks (spends) | Missile spends | Dry presses | Longest standstill |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `donut_prog_pilot_arena_boost` (direct, swap, `eng_solo_assist`, seeds 0..8; boost held) | 24 | 16 (the 8 `cf02_arena_swap` runs are a 300 tick script, not a clearing pilot) | 0 | 0 | 173 | 0 | 108 |
| `donut_prog_pilot_arena_blink` (`Pilot::ArenaDuoBlink`, all three capabilities) | 8 | 7 (seed 5 never) | 7 | 15 (15) | 81 | 16 | 173 |
| control: the shipped `ArenaDuo` pilot, same capabilities | 8 | 3 | 11 | 0 | 96 | 6 | 228 |
| `donut_prog_pilot_boss_boost` (solo and 2P full clear under the boss pin) | 2 | 2, both on tick 3175 with the Puddle Jumper grant on it | 0 | 0 | 0 | 0 | 788 (the parked P2, the same unpinned) |
| `cf02_free_blink_boss_arena` (AI Donut, boss pin and blink) | 1 | 1 (3175) | 0 | 0 | 0 | 0 | 376 |

No blink was cancelled or fizzled in any run. The blink pilot stands 32 px ahead of Carl (a recorded deviation: a trailing P2 is never the goblins' target). The Juicer's charge never makes the AI Donut blink in the boss arena: the jump dodge starts 18 ticks before the ram and the blink window opens at 12 (spec section 18).

### PLAYTRACE and the two-pad check

A `--cf02 --donut-puddle --autopilot --record` session (1500 ticks) records and verifies strict on the build that recorded it; its manifest carries the descriptor flags and `donut_prog_tuning`, but the autopilot drives Carl only, so its journal holds no blink event. The recordings of the story-run grants and of a session with blinks need a human on the pads: **NOT RUN**. The real two-pad check is **NOT RUN** (no second controller on the build machine). Recordings are not committed (size). The verify line of that session on the final head: `PLAYTRACE_VERIFY result=PASS scope=full ticks=1500 episodes=1 combat_events=54 control_events=6 build=identical`.
