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
| `carl_prog_story_event_grants_dynamite` | `7594ba644fd2dbaf` | one `capability_granted` (tick 69, hero 0, `special_thrown_charge`) on the first tick Carl's pivot is inside the placeholder zone of `pile_boss_f01_defeated` |
| `carl_prog_grant_survives_respawn` | `8eeb903687dec95d` | the grant at tick 69, a defeat at tick 495 and a party retry at tick 544 (twice): the capability stays held and is never granted a second time |
| `carl_prog_juicer_grants_fist` | `ddd681ab67f526ac` | under the boss pin the boss defeat (tick 311) and one `capability_granted` (`fist_reinforced`) share a tick; the pinned `special_thrown_charge` emits nothing |

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

## CP-B2 and CP-C evidence: the dynamite presentation and interactions

Real windowed Bevy captures (`floor01_scene`, 640 x 360 virtual frame, lit Floor 1 and the boss arena) of the
`carl_prog_*` fixtures, built by `bash scripts/make_cp_b2_evidence.sh` (a debug binary, the exported atlases,
`xvfb-run` and Pillow) into `cp-b2-c/`. GIFs are 10 or 20 fps by capture step; `-1x` is the whole frame,
`-3x` a nearest-neighbour crop around the action. The throw and stick art is
`prototype_fixture_unapproved` (its independent art review is pending); nothing here approves it.

| File | Shows |
| --- | --- |
| `cp-b2-c/throw-standing-{1x,3x}.gif` | `carl_prog_dyn_arc_standing` with `--debug-combat`: the standing ANIM-CARL-03 throw, the tumbling stick, the landing and fizzle, the 64 px radius ring, the VFX-01 burst. |
| `cp-b2-c/throw-moving-{1x,3x}.gif` | `carl_prog_dyn_arc_moving`: the moving overlay frames while Carl runs (the plain run frame for draw, brace and settle), then the longer arc. |
| `cp-b2-c/chain-{1x,3x}.gif` | `carl_prog_dyn_chain_depth_cap`: the stick, then charges lit 8 ticks apart (depth 1 to 4); the fifth never blows. |
| `cp-b2-c/shore-collapse-{1x,3x}.gif` | `carl_prog_dyn_breaks_shore`: the blast breaks the untouched shore, the block falls and crushes the goblin. |
| `cp-b2-c/wall-slam-{1x,3x}.gif`, `bowling-{1x,3x}.gif` | `carl_prog_dyn_wall_slam` and `carl_prog_dyn_bowling` (`cf02_rules`): a launched goblin meets the wall, a launched goblin bowls another. |
| `cp-b2-c/rack-strut-{1x,3x}.gif` | `carl_prog_dyn_rack_strut`: the blast drops the left rack on the Juicer outright. |
| `cp-b2-c/hud-socket-{locked,ready,live,recharging}.png` | Carl's plate at 4x: the padlock, the ready `B` glyph with `BOOM`, and the muted (live, recharging) socket. |
| `cp-b2-c/hud-grant-notice.png` | The whole frame on the grant tick of `carl_prog_story_event_grants_dynamite`: the equipped socket and the `GOBLIN BOOM` System notice. |
| `cp-b2-c/SHA256SUMS` | Checksums, verified by `python3 scripts/check_docs.py`. |

The solo and 2P Juicer dynamite recordings (`carl_prog_juicer_dynamite_clear`) are a follow-up. The replays of
the ten new fixtures are in `replays/`.
