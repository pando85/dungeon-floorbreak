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
