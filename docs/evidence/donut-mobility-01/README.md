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

GIFs 3 and 4 of the work order (the reflex, the microarena perch jump with the HUD drawn) are DM-C's,
below. DM-B's own evidence is the headless replays and the share table.

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

**DM-C update (F5, F6).** The tally now classifies a hurt by its source id and a test-only counter at
`Reflex::check` (the spec's definition of an attempt) must agree with it. A **human-style Donut**
population is added: `cf02_arena_coop` (P2 played as a human would), 48 seeds. The three solo pilots are
also run with `target_ai_bias_px` 0, but bias 0 changes nothing in these rooms: those rows equal the AI-Donut
rows, so they are labelled AI-Donut data and the combined line is not a human-style total. **The human-style
number is the coop row** (a small sample, 23 to 29 attempts). A swap-to-Donut pilot is a follow-up. Reading it,
on top of the AI-Donut rows below:

- **The AI-Donut rows measure the AI Donut, which the shipped rooms almost never attack.** With the
  human-style Donut (the coop row) the first attack on her arrives on tick 371 to 603 of an attempt, always
  before the 720-tick arm. So in A (biscuit from tick 0) it is **0 of 23 evaded at 720 and 900 (1 of 24 at 600)**:
  the reflex is not yet ready for the opener, and each attempt then restarts the wait.
- Pre-armed (B, the reflex ready when the room starts) the coop row, the human-style number, evades **10 of 29**
  (openers 10 of 24); the other attempts land while she re-arms (12 s after a hurt or an evade). The
  combined line (52 of 71) is mostly the AI Donut's Engineer ram, 42 of 42, and is not a human-style total. Rearm 600, 720 and 900 still give the same B table.
- So with a 12 s rearm the reflex is either not yet ready or swallows the opener of the next wave
  (spec U1). This is a measurement for the owner's playtest (`--cf02 --donut-biscuit`), not a tuning:
  720 stays the default.

- **A** (biscuit from tick 0, the spec's population): **0 evades**. The only evadable attack that reaches
  the AI Donut in the shipped rooms is the Engineer's ram (the microarena goblins and llama never
  target her), first on tick 427 to 487 of the attempt, which is inside even the shortest rearm.
- **B** (biscuit held and 12 s or more hurt-free before the room): every attempt on her is evaded (3 of 3 in
  the fixtures, 42 of 42 in the pilots), and every pilot one is the **opener of the second wave** (U1). Rearm 600, 720 and 900 give the same
  table, because no run has a second evadable attack on her inside 1800 ticks.
- Sample: the population rows hold 3 attempts ("insufficient sample", the shipped fixtures almost never
  attack Donut); the pilot rows hold 42 (48 seeds each), all of them from `eng_solo_assist`.
  `cf02_arena_direct` and `cf02_arena_swap` never attack her.

# DM-C evidence (the cue stand-ins)

Real Bevy renders (`floor01_scene`, 640x360, lighting on) of two new evidence rooms, plus the DM-A lane
GIFs for work-order GIFs 1 and 2. **Presentation only, no new art**: the cues are code-drawn overlays on
her existing approved frames (spec section 3.4, section 11 of the spec). They show what the stand-ins
look like; they are not a game-feel verdict and are not reproduced bit for bit by CI (driver
dependent). The deterministic half is pinned by the tests and the headless replays below.

| File | What it shows |
| --- | --- |
| [`dm_a_run_race_1x.gif`](dm_a_run_race_1x.gif) | GIF 1 (DM-A, unchanged: no biscuit, so no cue): both run 120 ticks, she ends 44 px ahead |
| [`dm_a_jump_race_1x.gif`](dm_a_jump_race_1x.gif) | GIF 2 (DM-A, unchanged, 1x crop): the jump-height race in the open hall, a standing and a running jump side by side (apex and distance). There is no 96 px ledge in it; the ledge landings are proven by the DM-A geometry tests. GIFs 1 and 2 are 1x crops, not the 2x of spec section 7 |
| [`dm_c_gif3_reflex.gif`](dm_c_gif3_reflex.gif) | GIF 3, `cf02_dm_reflex_cues`, 2x, half speed: the glint blinks while she is armed (arms 720), the goblin swings (729), the evade (arc, two afterimages, the white streak), she steps back, smug (756..775), the glint is gone; then "12 s later", ticks 1446 to 1486, armed again (1449) and the glint is back |
| [`dm_c_gif4_perch_jump_hud.gif`](dm_c_gif4_perch_jump_hud.gif) | GIF 4, `cf02_arena_dm_perch_jump`, 2x, the HUD drawn: a running jump onto the left perch, then a standing jump under the DM-Q27 lane ceiling (bonk at feet 176); her drawn top stays under the HUD band |
| [`dm_c_cues_sheet.png`](dm_c_cues_sheet.png) | The contact sheet: one panel per cue tick at 3x (not armed, glint and flick, glint off, the evade at ages 0, 3, 6 and 11, smug, spent, armed again) |
| [`replays/headless_cf02_dm_reflex_cues.txt`](replays/headless_cf02_dm_reflex_cues.txt) | The reflex room's replay (`evade_armed` 720, `hero_evaded` 729, `evade_armed` 1449, nobody hurt), reproduced by `scripts/ci_floor01_combat.sh headless` |
| [`replays/headless_cf02_arena_dm_perch_jump.txt`](replays/headless_cf02_arena_dm_perch_jump.txt) | The perch room's replay |

Recipe (from the repository root, after `bash scripts/export_assets.sh` and a debug build): `bash
scripts/make_dm_c_evidence.sh`. The glint is one 2 x 2 square in her atlas's brightest colour, so on
her pale tail highlights it reads best in motion (the 4-on, 26-off blink) and on the sheet.

Run cycle and jump phases (spec 3.4) are checked by `agility_presentation_tests`: at 207 px/s her run
frames last 71, 71, 71, 71, 80 and 71 ms, each covering its authored ground distance within 0.2 px;
at the 500 px/s launch and through the DM-Q27 bonk the phases sequence `jump_start`, `jump_rise`,
`jump_fall`, `land` with none repeated or skipped.
