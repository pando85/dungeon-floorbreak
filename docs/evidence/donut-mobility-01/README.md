# DONUT-MOBILITY-01 DM-A evidence

Real Bevy renders (`floor01_scene`, 640x360, 1x, lighting on) of the two side-by-side fixtures of
`src/agility_fixtures.rs`, cropped to the lane (640x230). They show what the motion looks like;
they are not a game-feel verdict and are not reproduced bit for bit by CI (driver dependent).
The same fixtures are pinned headlessly by `agility_fixtures::tests`.

| GIF | Fixture | What it shows |
| --- | --- | --- |
| [`dm_a_run_race_1x.gif`](dm_a_run_race_1x.gif) | `dm_run_race_2p`, ticks 4..124 | Both heroes start in Carl's column and run right for 120 ticks: Donut (207 px/s) ends 44 px ahead of Carl (185 px/s) |
| [`dm_a_jump_race_1x.gif`](dm_a_jump_race_1x.gif) | `dm_jump_race_2p`, ticks 6..170 | Donut starts 60 px behind Carl; a standing jump and then a running jump: her apex is 114.92 px against his 88.54, her flight is 57 ticks against 50 |

Recipe (from the repository root, after `bash scripts/export_assets.sh` and a debug build):

```sh
xvfb-run -a -s '-screen 0 1920x1200x24' ./target/debug/floor01_scene --scale 1 \
  --party-fixture dm_run_race_2p --capture-sequence 4:2:61 --capture-dir <scratch>/race
xvfb-run -a -s '-screen 0 1920x1200x24' ./target/debug/floor01_scene --scale 1 \
  --party-fixture dm_jump_race_2p --capture-sequence 6:2:82 --capture-dir <scratch>/jump
magick -delay 3 -loop 0 <scratch>/race/frame_*.png -crop 640x230+0+100 +repage -colors 96 \
  -fuzz 3% -layers OptimizeTransparency dm_a_run_race_1x.gif
```

The jump GIF uses the same `magick` line on its frames. Checksums: `SHA256SUMS`.

GIFs 3 and 4 of the work order (the reflex, the microarena perch jump with the HUD drawn) belong to
DM-C: the reflex has no presentation yet, so DM-B's evidence is the headless replays and the share
table below.

# DM-B evidence (reflex evasion, no presentation)

Headless, deterministic, byte-reproduced by `bash scripts/ci_floor01_combat.sh headless` (each replay
runs twice and is diffed against the committed one).

| File | What it shows |
| --- | --- |
| [`replays/headless_dm_reflex_evades_swing.txt`](replays/headless_dm_reflex_evades_swing.txt) | Biscuit from tick 0, P2 Donut walks up to a sleeping goblin. `evade_armed` on tick 720, the first swing on tick 729 is evaded (`hero_evaded`, no `hero_hurt`, health unchanged), the next swing on tick 820 lands |
| [`replays/headless_dm_reflex_not_armed.txt`](replays/headless_dm_reflex_not_armed.txt) | The same walk at tick 20: the swing on tick 129 lands, no `evade_armed` and no `hero_evaded` (300 ticks, long before 720) |
| [`replays/headless_dm_evasion_locked_before_biscuit.txt`](replays/headless_dm_evasion_locked_before_biscuit.txt) | No biscuit, 900 hurt-free ticks, then the swing on tick 909 lands: the reflex is a story unlock and nothing else |
| [`replays/headless_dm_evasion_unlocks_at_guild.txt`](replays/headless_dm_evasion_unlocks_at_guild.txt) | Locked at the start; the guild zone grants the biscuit on tick 20 (`unlock_granted`), `evade_armed` on tick 740 = 20 + 720, the swing on tick 741 is evaded |
| [`dm_b_reflex_share.txt`](dm_b_reflex_share.txt) | The measured evade share (spec section 3.3, no target) at rearm 600, 720 and 900, with the per-wave and per-attempt counts and the opener counts |

The ram (`dm_reflex_ram_passes`: Donut is evaded and Carl behind her is hit on the same tick 44) and the
glob (`dm_reflex_glob_passes`: the glob passes through her jumping and hurts Carl) need `cf02_rules` and
the arena kit, which the binary keys by fixture-name prefix, so they are test-only
(`combat_scene/evade/tests.rs`); their events are asserted there.

Regenerate the share table: `DM_B_SHARE_OUT=docs/evidence/donut-mobility-01/dm_b_reflex_share.txt
cargo test --locked --lib the_evade_share`. Read it as a measurement, not a tuning target (owner
decision: no share target, 720 ticks stays the default whatever it shows):

- **A** (biscuit from tick 0, the spec's population): **0 evades**. The only evadable attack that reaches
  the AI Donut in the shipped rooms is the Engineer's ram (the microarena goblins and llama never
  target her), first on tick 427 to 487 of the attempt, which is inside even the shortest rearm.
- **B** (biscuit held and 12 s or more hurt-free before the room): every attempt on her is evaded (3 of 3 in
  the fixtures, 42 of 42 in the pilots), and every pilot one is the **opener of the second wave** (U1). Rearm 600, 720 and 900 give the same
  table, because no run has a second evadable attack on her inside 1800 ticks.
- Sample: the population rows hold 3 attempts ("insufficient sample", the shipped fixtures almost never
  attack Donut); the pilot rows hold 42 (48 seeds each), all of them from `eng_solo_assist`.
  `cf02_arena_direct` and `cf02_arena_swap` never attack her.
