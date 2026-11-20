# CF02-H evidence

Replays, not captures: the fourteen `cf02_*` fixtures of CF02-H are in [`../replays/`](../replays/) (run twice by `scripts/ci_floor01_combat.sh headless` and pinned by `GOLDENS` in `src/cf02_fixtures.rs`). The air kick, dive and bail-out draw the existing `kick` frames (placeholders; CF02-I wires the final frames).

| Beat (design question) | Fixture | What the replay shows |
| --- | --- | --- |
| Jump-in kick (Q50) | `cf02_air_kick_jump_in` | air kick on the way up hits on tick 8: stagger 14, knock 90 |
| Relaunch (Q51) | `cf02_air_kick_relaunch_once` | a flying goblin is relaunched at 150 / 130 once |
| Dive (Q52, Q53) | `cf02_dive_lying`, `cf02_dive_standing_knockdown`, `cf02_dive_bounce_kick`, `cf02_dive_whiff_recovery`, `cf02_dive_boss` | tuck 6, plunge, knockdown or in-place finish, bounce (vy 240), the period's one air kick, 2-damage cap, 10-tick whiff recovery, boss armour |
| Bail-out (Q54, Q56) | `cf02_bailout_escapes_swing`, `cf02_bailout_no_attack_until_land` | Jump in a jab's recovery 6 ticks before impact escapes (measured boundary below); no attack until landing, 4-tick recovery |
| Buffer and same tick (Q46..Q48) | `cf02_buffer_turns`, `cf02_buffer_no_early_chain`, `cf02_jump_attack_same_tick`, `cf02_landing_grace` | buffered kick turns Carl; no chain from jab tick 1; Jump + Physical is a jump kick; Down + Physical near the floor is the ground stomp |
| Cancel table (Q49) | `cf02_cancel_table` | C4 (air kick that hit, then Down + Physical in its recovery) |

**Measured bail-out boundary (deviation for the owner's game-feel call):** the acceptance says the bail-out escapes a tell seen 5 or more ticks before impact and not later. Measured with the goblin's swing landing on tick 37: a Jump 7, 6, 5 **and 4** ticks before impact escapes; **3** ticks before is hit (`q54_the_bailout_escapes_a_tell_seen_4_or_more_ticks_before_impact_and_not_later`). So the real window is one tick more forgiving than the wording. No knob was changed; the owner may tighten `q54_bailout_vx` / `q54_bailout_vy` if it feels too safe.

**C4 early press (N1):** a Down + Physical pressed on any tick 10..=18 of the `cf02_cancel_table` geometry (the air kick's hit-stop and active ticks) is remembered and fires the dive on the first tick the recovery opens (`q49_n1_a_down_physical_pressed_during_the_air_kick_takes_c4_in_the_recovery`).

C1, C2, C3 and C5 are shown by `cf02_jab_over_lying` (C1), `cf02_confirm_kick` (C2), `cf02_bailout_escapes_swing` (C3) and `cf02_dive_bounce_kick` (C5). The unit test `q49_no_press_cancels_a_ground_move_outside_the_table` sweeps every press at every tick of the jab, kick and stomp and requires that nothing but C1 starts a move early.

## Input-to-move latency (ticks from the press tick, 60 Hz)

| Input | First active tick | Notes |
| --- | --- | --- |
| Primary (jab) | 3 | unchanged |
| Physical (kick) | 13 | unchanged |
| Down + Physical on the ground (stomp) | 9 | Q02 |
| Physical in the air (air kick) | 5 | Q50 |
| Down + Physical in the air (dive) | 6 (tuck), plunge on 6, hit about tick 9 from the apex | Q52 |
| Jump in a grounded move's recovery (bail-out) | 0 (the hop starts the same tick) | Q54 |

Buffer 6 vs 8 (`q46_the_window_is_8_under_cf02_and_6_in_the_legacy_game`): a Physical pressed up to 8 ticks before a jab ends starts the kick under `--cf02`; up to 6 ticks in the legacy game.

**NOT RUN:** the 1x GIF of the jump-in kick, the dive from a raised block and the bail-out (no windowed capture was produced in this PR).
