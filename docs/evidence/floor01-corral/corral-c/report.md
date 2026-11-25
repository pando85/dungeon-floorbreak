# CORRAL-AUTO report: the Infernal Corral (after) and the corridor baselines (before)

Policy version 2, horizon 1800 ticks, 21384 runs. The unit is a fixture (start x surface x facing x attack phase x reflex x seat x policy), never a seed: seeds do not vary gameplay. The corral stages run on `f01_corral.room.json` with the CORRAL-B rush and llama party targeting; the baselines run on the corridor kit with today's legacy llamas (no rush, no cover). Partition variants `intact` and `broken` are `not_run` until CORRAL-D.

## Corral gate (asserted)

| Gate | Outcome |
| --- | --- |
| M1 positive control: `stationary_cast_aimed` wins M1-A and M1-B with 0 damage | pass |
| KC19 on M2, decisive variant (Carl parked, reflex on) | pass |
| CRL-Q35 invariant: no gated static run defeats both M2 llamas by Donut hits | pass |
| A mobile Donut victory on M2 | pass |
| A Carl melee victory on M2 | pass |
| A 2P co-op victory on M2 | pass |
| No hard system failure on any corral run | pass |
| KC19 on M2 with the reflex off (reported) | pass |

KC19 groups judged: 2122; failing: 0. `layer_static_clear` (diagnostic, not gated): 3204 of 12732 gated parked M2 runs defeated the llama of their own level.

## Baselines (report-only, the "before")

| Gate | Outcome |
| --- | --- |
| M1 positive control | pass |
| M2 stationary KC19, reflex off | fail |
| M2 stationary KC19, reflex on | fail |
| Mobile Donut victory on M2 | pass |
| Carl melee victory on M2 | pass |
| 2P co-op victory on M2 | pass |
| Mana accounting closes on every run | pass |
| `hold_attack_trivial` is one press | pass |

Baseline Carl-only llama targeting (legacy llamas; measured, not asserted): present (every legacy llama aims at Carl).

Best baseline counterexample: `corral_eval_m2.solo_donut_bot.stationary_cast.off.x560.h0.fr.p0-0-45-0` (105 ticks, Donut lost 0 HP).

## Tallies

| Stage | Seat | Policy | Evasion | Runs | Victory | Low-risk | No-damage | Defeat | Timeout | Invalid | Companion-assisted | Win ticks |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| corral_eval_m1 | solo_carl | carl_melee | off | 3 | 3 | 3 | 2 | 0 | 0 | 0 | 0 | 129-215 |
| corral_eval_m1 | solo_carl | carl_melee | reflex | 3 | 3 | 3 | 2 | 0 | 0 | 0 | 0 | 129-215 |
| corral_eval_m1 | solo_donut_assist | stationary_cast | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_assist | stationary_cast | reflex | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_assist | stationary_cast_burst | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-546 |
| corral_eval_m1 | solo_donut_assist | stationary_cast_burst | reflex | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-546 |
| corral_eval_m1 | solo_donut_bot | hold_attack_trivial | off | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | hold_attack_trivial | reflex | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | mobile_cast | off | 54 | 49 | 49 | 32 | 0 | 5 | 0 | 0 | 57-334 |
| corral_eval_m1 | solo_donut_bot | mobile_cast | reflex | 54 | 48 | 48 | 32 | 0 | 6 | 0 | 0 | 57-334 |
| corral_eval_m1 | solo_donut_bot | no_interaction | off | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | no_interaction | reflex | 54 | 0 | 0 | 0 | 0 | 54 | 0 | 0 | - |
| corral_eval_m1 | solo_donut_bot | stationary_cast | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_bot | stationary_cast | reflex | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-456 |
| corral_eval_m1 | solo_donut_bot | stationary_cast_burst | off | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-546 |
| corral_eval_m1 | solo_donut_bot | stationary_cast_burst | reflex | 54 | 27 | 27 | 19 | 0 | 27 | 0 | 0 | 43-546 |
| corral_eval_m1 | two_player | coop_probe | off | 54 | 54 | 54 | 42 | 0 | 0 | 0 | 0 | 43-215 |
| corral_eval_m1 | two_player | coop_probe | reflex | 54 | 54 | 54 | 42 | 0 | 0 | 0 | 0 | 43-215 |
| corral_eval_m2 | solo_carl | carl_melee | off | 9 | 9 | 9 | 0 | 0 | 0 | 0 | 0 | 260-450 |
| corral_eval_m2 | solo_carl | carl_melee | reflex | 9 | 9 | 9 | 0 | 0 | 0 | 0 | 0 | 260-450 |
| corral_eval_m2 | solo_donut_assist | stationary_cast | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_assist | stationary_cast | reflex | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_assist | stationary_cast_burst | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-603 |
| corral_eval_m2 | solo_donut_assist | stationary_cast_burst | reflex | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-603 |
| corral_eval_m2 | solo_donut_bot | hold_attack_trivial | off | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | hold_attack_trivial | reflex | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | mobile_cast | off | 162 | 111 | 111 | 20 | 2 | 49 | 0 | 0 | 113-526 |
| corral_eval_m2 | solo_donut_bot | mobile_cast | reflex | 162 | 112 | 112 | 20 | 0 | 50 | 0 | 0 | 113-526 |
| corral_eval_m2 | solo_donut_bot | no_interaction | off | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | no_interaction | reflex | 162 | 0 | 0 | 0 | 0 | 162 | 0 | 0 | - |
| corral_eval_m2 | solo_donut_bot | stationary_cast | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_bot | stationary_cast | reflex | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-612 |
| corral_eval_m2 | solo_donut_bot | stationary_cast_burst | off | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-603 |
| corral_eval_m2 | solo_donut_bot | stationary_cast_burst | reflex | 162 | 72 | 72 | 36 | 0 | 90 | 0 | 0 | 105-603 |
| corral_eval_m2 | two_player | coop_probe | off | 162 | 162 | 162 | 28 | 0 | 0 | 0 | 0 | 98-460 |
| corral_eval_m2 | two_player | coop_probe | reflex | 162 | 162 | 162 | 28 | 0 | 0 | 0 | 0 | 98-460 |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast_burst | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_assist | stationary_cast_burst | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | hold_attack_trivial | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | hold_attack_trivial | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | mobile_cast | off | 126 | 91 | 91 | 16 | 0 | 35 | 0 | 0 | 118-545 |
| corral_eval_m2_pincer | solo_donut_bot | mobile_cast | reflex | 126 | 85 | 85 | 16 | 0 | 41 | 0 | 0 | 118-546 |
| corral_eval_m2_pincer | solo_donut_bot | no_interaction | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | no_interaction | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast_burst | off | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | solo_donut_bot | stationary_cast_burst | reflex | 126 | 0 | 0 | 0 | 0 | 126 | 0 | 0 | - |
| corral_eval_m2_pincer | two_player | coop_probe | off | 126 | 125 | 125 | 38 | 1 | 0 | 0 | 0 | 112-412 |
| corral_eval_m2_pincer | two_player | coop_probe | reflex | 126 | 125 | 125 | 38 | 1 | 0 | 0 | 0 | 112-412 |
| corral_m1 | solo_donut_parked | stationary_cast | off | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m1 | solo_donut_parked | stationary_cast | reflex | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m1 | solo_donut_parked | stationary_cast_aimed | off | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m1 | solo_donut_parked | stationary_cast_aimed | reflex | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m1 | solo_donut_parked | stationary_cast_burst | off | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m1 | solo_donut_parked | stationary_cast_burst | reflex | 4 | 2 | 2 | 2 | 0 | 2 | 0 | 0 | 69-76 |
| corral_m2 | solo_carl | carl_melee | off | 6 | 6 | 6 | 6 | 0 | 0 | 0 | 0 | 449-486 |
| corral_m2 | solo_carl | carl_melee | reflex | 6 | 6 | 6 | 6 | 0 | 0 | 0 | 0 | 449-486 |
| corral_m2 | solo_donut_assist | stationary_cast | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast_burst | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast_burst | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | hold_attack_trivial | off | 354 | 0 | 0 | 0 | 249 | 105 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | hold_attack_trivial | reflex | 354 | 0 | 0 | 0 | 249 | 105 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | mobile_cast | off | 72 | 72 | 72 | 72 | 0 | 0 | 0 | 0 | 172-368 |
| corral_m2 | solo_donut_parked | mobile_cast | reflex | 72 | 72 | 72 | 72 | 0 | 0 | 0 | 0 | 172-368 |
| corral_m2 | solo_donut_parked | no_interaction | off | 354 | 0 | 0 | 0 | 252 | 102 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | no_interaction | reflex | 354 | 0 | 0 | 0 | 252 | 102 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_aimed | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_aimed | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_burst | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_burst | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast | off | 4596 | 0 | 0 | 0 | 887 | 3709 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast | reflex | 4596 | 0 | 0 | 0 | 881 | 3715 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast_aimed | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast_aimed | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_turret | off | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_turret | reflex | 354 | 0 | 0 | 0 | 0 | 354 | 0 | 0 | - |
| corral_m2 | two_player | coop_probe | off | 72 | 34 | 34 | 24 | 0 | 38 | 0 | 0 | 213-651 |
| corral_m2 | two_player | coop_probe | reflex | 72 | 34 | 34 | 24 | 0 | 38 | 0 | 0 | 213-651 |
