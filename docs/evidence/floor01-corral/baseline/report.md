# CORRAL-EVAL baseline report (reflex absent)

Policy version 1, horizon 1800 ticks, 5496 runs. The unit is a fixture (start x facing x attack phase), never a seed: seeds do not vary gameplay. Stages are baselines on the corridor kit with today's llamas (no rush, no party targeting, no cover), not the Infernal Corral. The reflex variant is `DM-B reflex is not on main (PR #199); DM-A declares dm_q11_evasion_mode but nothing reads it; variant is a parameter only`.

## Gates

| Gate | Outcome | Status |
| --- | --- | --- |
| M1 positive control (stationary Donut wins M1) | pass | asserted |
| Mana accounting closes on every run | pass | asserted |
| `hold_attack_trivial` is one press | pass | asserted |
| M2 stationary gate, KC19, reflex off | fail | REPORT-ONLY until CORRAL-C |
| M2 stationary gate, KC19, reflex on | blocked | REPORT-ONLY, blocked until DM-B |
| Mobile Donut victory on M2 | pass | REPORT-ONLY until CORRAL-C |
| Carl melee victory on M2 | pass | REPORT-ONLY until CORRAL-C |
| 2P co-op victory on M2 | pass | REPORT-ONLY until CORRAL-C |

Carl-only llama targeting (a hard system failure once CORRAL-B lands; measured, not asserted): present (every llama aims at Carl).

M2 stationary groups judged: 64; failing: 16.

Best counterexample (fewest ticks, then least Donut HP lost, over all failing groups): `corral_eval_m2.solo_donut_bot.stationary_cast.off.x560.fr.p0-45` (105 ticks, Donut lost 0 HP).

## Tallies

| Stage | Seat | Policy | Evasion | Runs | Victory | Low-risk | No-damage | Defeat | Timeout | Softlock | Blocked | Invalid | Companion-assisted | Win ticks |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| corral_eval_m1 | solo_carl | carl_melee | off | 3 | 3 | 3 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 211-258 |
| corral_eval_m1 | solo_donut_assist | stationary_cast | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_assist | stationary_cast_burst | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 0 | 0 | 43-546 |
| corral_eval_m1 | solo_donut_bot | hold_attack_trivial | off | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | mobile_cast | off | 54 | 49 | 49 | 32 | 0 | 5 | 0 | 0 | 0 | 0 | 57-334 |
| corral_eval_m1 | solo_donut_bot | no_interaction | off | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | stationary_cast | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_bot | stationary_cast_burst | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 0 | 0 | 43-546 |
| corral_eval_m1 | two_player | coop_probe | off | 54 | 54 | 54 | 17 | 0 | 0 | 0 | 0 | 0 | 0 | 43-258 |
| corral_eval_m2 | solo_carl | carl_melee | off | 9 | 7 | 7 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 475-527 |
| corral_eval_m2 | solo_donut_assist | stationary_cast | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_assist | stationary_cast_burst | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 0 | 0 | 105-603 |
| corral_eval_m2 | solo_donut_bot | hold_attack_trivial | off | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | mobile_cast | off | 162 | 111 | 111 | 20 | 2 | 49 | 0 | 0 | 0 | 0 | 113-526 |
| corral_eval_m2 | solo_donut_bot | no_interaction | off | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | stationary_cast | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_bot | stationary_cast_burst | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 0 | 0 | 105-603 |
| corral_eval_m2 | two_player | coop_probe | off | 162 | 134 | 134 | 18 | 28 | 0 | 0 | 0 | 0 | 0 | 105-527 |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast_burst | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | hold_attack_trivial | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | mobile_cast | off | 126 | 91 | 91 | 16 | 0 | 35 | 0 | 0 | 0 | 0 | 118-545 |
| corral_eval_m2_pincer | solo_donut_bot | no_interaction | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast_burst | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | 0 | 0 | - |
| corral_eval_m2_pincer | two_player | coop_probe | off | 126 | 126 | 126 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 211-424 |

2748 runs of the `reflex` variant are `blocked` (not on main) and are left out of the table; they stay in `runs.csv`.
