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

## CP-E evidence: pierce, one-combo shore, the 64 px reading, the Juicer, and the 640 x 400 view

Real windowed captures (`floor01_scene`, `--party-fixture`, 640 x 360 and, with `--deck`, 640 x 400) built by
`bash scripts/make_cp_e_evidence.sh` (a debug binary, the exported atlases, `xvfb-run`, Pillow) into `cp-e/`. GIFs play at
game speed (the frame delay is the capture step in ticks at 60 Hz); `-3x` is a nearest-neighbour enlargement of a crop.
Every sequence is also replayed headless to its last tick and the two final `COMBAT_STATE` hashes are equal
(`cp-e/capture_lines.txt`, `match=yes`): the presentation is read-only. Art stays `prototype_fixture_unapproved`; nothing
here approves it and no game-feel claim is made.

**Pierce spark (the CP-D carry-over, new here).** The reinforced fist's pierce of a committed attack
(`CombatState::pierce_cue`, CP-Q42) now draws the shared `vfx_hit_spark_metal` atlas (3 frames, 2 ticks each) once at the
contact face of the pierced goblin or llama (`src/bin/floor01_scene/pierce_spark.rs`, one pre-spawned sprite; pure frame
rule `blast_art::spark_frame`). It reads the cue only: nothing is hashed, no replay or golden changed.

| File | Shows |
| --- | --- |
| `cp-e/pierce-goblin-{1x,3x}.gif`, `pierce-goblin-640x400-{1x,3x}.gif` | `carl_prog_pierce_goblin` (the fixture holds the gauntlet): the goblin raises its club (tick 40 tell); the haymaker lands on attack tick 24 (tick 65), the swing is cancelled and the goblin staggers. A small white star shows at the goblin's face on ticks 65 to 67 and fades. |
| `cp-e/pierce-goblin-control-{1x,3x}.gif` | `carl_prog_pierce_goblin_control`: the same input without the gauntlet; the swing runs on and no spark is drawn. |
| `cp-e/shore-one-combo-{1x,3x}.gif` | `carl_prog_shore_combo`: a jab (tick 23, one hit) then the reinforced haymaker (which counts two) break the shore in one combo (tick 32); the block falls and crushes the goblin (tick 68). |
| `cp-e/reach-ring-outer-launch-{1x,3x}.gif`, `reach-ring-wall-slam-{1x,3x}.gif` | The **64 px reading** (below): `--debug-combat` draws the dotted outer-radius ring and the hurtboxes while the stick is lit, then the burst. `carl_prog_dyn_outer_launch` (a goblin 40 px from the blast is launched alive with 1 HP) and `carl_prog_dyn_wall_slam` (CF02 room, the launched goblin meets the wall, tick 198). |
| `cp-e/throw-standing-640x400-{1x,3x}.gif` | The standing throw in the 640 x 400 deck view. |
| `cp-e/juicer-dynamite-{1x,3x}.gif` | `carl_prog_juicer_dynamite_clear` (solo): Carl throws at the Juicer, the stick lands (tick 212) and blows at 324 while the Juicer pounds Donut. |

Already in `cp-b2-c/` and not repeated: the standing and moving throws (the moving throw also shows the self-damage:
`carl_prog_dyn_arc_moving`, `hero_hurt` on the blast tick 196), the chain into an authored charge, the shore broken by a
blast, the wall slam, the grant notice and the HUD socket.

**Owner handoff (game feel, nothing here decides it).**

- **64 px reading.** The owner's "64 px radius" is a 20 px lethal core plus a 64 px ring that deals 1 damage and a launch
  (CP-Q20, CP-Q21): against goblins the dynamite mostly launches and kills only within 20 px. The VFX-01 burst is a roughly
  20 px fireball (48 px frame); in `reach-ring-*` the dotted ring is visibly wider than the burst. Is the reach readable
  without the debug ring, or does it need `burst_large` or a player-visible ring (CP-Q33)?
- **Run-persistence.** "Permanently" is run-persistent (CP-Q07): a fixture restart (R, or Select on a pad) restores the pin
  and removes a grant earned in play.
- **Running throw.** A running stick lands about 170 px ahead and Carl, still running, passes it near tick 55, about 95
  ticks before it blows: a forward throw on the run buys no space (`throw-moving` in `cp-b2-c`).
- **Pierce scope (F-12).** The shipped default pierces on the haymaker only; the jab pierce is one data row (`cpq40_jab_pierces_commit`, shown by `carl_prog_pierce_goblin_jab`). Whether the jab should pierce too is the owner's call. The spark is drawn for whichever punch pierces.

**NOT RUN.**

- **2P Juicer with dynamite: NOT CAPTURED.** No two-player Juicer dynamite fixture exists (`carl_prog_dyn_no_friendly_fire` is the only 2P `carl_prog_*` fixture, in the arena). The solo Juicer clip above is the recording.
- **Frame time at gore preset 4: NOT RUN.** The captures use the xvfb software rasterizer, so no frame-time number is claimed.
- **A real two-pad check: NOT RUN.** No pad was used.
- The 3x crops were chosen by hand for the arena camera; a different camera setting moves the action out of them.
