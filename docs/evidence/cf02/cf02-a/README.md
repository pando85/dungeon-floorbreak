# CF02-A evidence

Replays, not captures: the headless `--party-fixture` replays of the thirteen `cf02_*` fixtures of CF02-A are in [`../replays/`](../replays/), each run twice by `scripts/ci_floor01_combat.sh headless` and pinned by `GOLDENS` in `src/cf02_fixtures.rs`.

| Beat (design question) | Fixture | What the replay shows |
| --- | --- | --- |
| Kick, walk, ground finish (Q01, Q02, Q04) | `cf02_kick_then_stomp` | kick hit on tick 18, goblin lies from about tick 43, stomp pressed 66 hits 75 and finishes it in place as a heavy defeat |
| On-the-spot stomp (Q02) | `cf02_stomp_standing`, `cf02_stomp_whiff` | stagger 10 with no knock; a whiff plays 26 ticks and steps 6 px |
| Follow-through (Q05, Q35) | `cf02_confirm_kick`, `cf02_whiff_no_confirm`, `cf02_down_physical_beats_confirm` | the follow-through starts on jab tick 9 only after a hit; Down + Physical is always the stomp |
| Lying goblin (Q03, Q04, Q34) | `cf02_jab_over_lying`, `cf02_stomp_no_stand_up` | jab and haymaker pass over the 36 x 20 lying box; a non-lethal stomp keeps it down |
| Lethal kick (Q06) | `cf02_lethal_kick_far` | the body flies about 125 px |
| Donut hold (Q34, Q38) | `cf02_pounce_keeps_down`, `cf02_pounce_hold` | a non-lethal pounce keeps the goblin down and holds its timer 20 ticks, once per down |
| Boss and 2P (Q08, Q35) | `cf02_stomp_boss`, `cf02_2p_same_tick` | boss super armour holds; Donut ignores Down |

The stomp and the follow-through draw the existing `kick` frames (placeholders, labelled; CF02-I wires the final frames).

**NOT RUN:** debug-box captures against the Q02/Q03 boxes and the 1x GIF of the three branches (no windowed capture was produced in this PR; the boxes are asserted numerically by `q03_the_lying_hurtbox_is_36_by_20_only_while_lying` and `cf02_a_moves_resolve_by_id_and_carry_the_data_file_values`).
