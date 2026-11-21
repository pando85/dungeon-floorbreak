# CARL-PROG-01 CP-A evidence: the story-unlock ledger

Headless replays of the `carl_prog_*` fixtures (`replays/headless_<fixture>.txt`, the
`COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of
`floor01_scene --headless-floor01-smoke --party-fixture <name>`). Each replays twice
identically, equals its committed file (unit test and the `headless` loop of
`scripts/ci_floor01_combat.sh`) and has its golden pinned in `src/carl_prog_fixtures.rs`.
Placeholder status: the real find, Hoarder and Juicer content is later work, so the
fixtures use a placeholder zone and the boss arena. A render-loop replay of these
fixtures is added by the first CARL-PROG PR that needs one (CP-B2); CP-A has no
presentation.

| Fixture | Final `PARTY_STATE` hash | What it shows |
| --- | --- | --- |
| `carl_prog_story_event_grants_dynamite` | `fcce3cba94bd81af` | one `capability_granted` (tick 69, hero 0, `special_thrown_charge`) on the first tick Carl's pivot is inside the placeholder zone of `pile_boss_f01_defeated` |
| `carl_prog_grant_survives_respawn` | `4aeab69b4200603b` | the grant at tick 69, a defeat at tick 495 and a party retry at tick 544 (twice): the capability stays held and is never granted a second time |
| `carl_prog_juicer_grants_fist` | `3f289ce118dca4b8` | under the boss pin the boss defeat (tick 311) and one `capability_granted` (`fist_reinforced`) share a tick; the pinned `special_thrown_charge` emits nothing |

Event log of a grant and a defeat retry (`carl_prog_grant_survives_respawn`):

```
COMBAT_EVENT tick=69 kind=capability_granted hero=0 capability=special_thrown_charge
COMBAT_EVENT tick=495 kind=hero_defeated
COMBAT_EVENT tick=544 kind=restart deaths=1
COMBAT_EVENT tick=495 kind=hero_defeated
COMBAT_EVENT tick=544 kind=restart deaths=2
```

The existing goldens are untouched: every earlier fixture, `EXPECTED_HASHES`, the boss,
Donut, CF02, companion and Engineer goldens and the PLAYTRACE tests pass unchanged, and
`bash scripts/ci_floor01_combat.sh headless` reproduces every committed replay.
