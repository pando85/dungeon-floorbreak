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
