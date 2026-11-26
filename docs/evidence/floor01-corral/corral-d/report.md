# CORRAL-AUTO report: the Infernal Corral (after) and the corridor baselines (before)

Policy version 3, horizon 1800 ticks, 53652 runs. The unit is a fixture (start x surface x facing x attack phase x reflex x seat x policy), never a seed: seeds do not vary gameplay. The corral stages run on `f01_corral.room.json` with the CORRAL-B rush and llama party targeting; the baselines run on the corridor kit with today's legacy llamas (no rush, no cover). Partition variants (CORRAL-D): `absent` is the cut path, `intact` the shipped room and `broken` the pre-broken remnant; the counts are in the table below.

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

KC19 groups judged: 6460; failing: 0. `layer_static_clear` (diagnostic, not gated): 9090 of 38760 gated parked M2 runs defeated the llama of their own level.

Partition variants (CORRAL-D), corral M2 runs, KC19 judged on the decisive variant (Carl parked, reflex on):

| Variant | Runs | Invalid starts | Victories | KC19 groups | KC19 failing | CRL-Q35 breaches | Partition breaks (rush / hero hits) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| absent | 15864 | 0 | 224 | 2122 | 0 | 0 | 0 / 0 |
| broken | 15732 | 0 | 148 | 2108 | 0 | 0 | 0 / 0 |
| intact | 16536 | 0 | 108 | 2230 | 0 | 0 | 2206 / 0 |

`layer_static_clear` by start surface (open question 3, owner-level):

| Surface | Gated parked runs | Clears | B defeated by Donut | A defeated by Donut |
| --- | ---: | ---: | ---: | ---: |
| bumper | 2196 | 0 | 0 | 0 |
| dock | 6156 | 1980 | 0 | 1980 |
| floor | 24108 | 7068 | 7068 | 0 |
| partition | 816 | 0 | 0 | 0 |
| remnant | 84 | 42 | 42 | 0 |
| shelf | 5400 | 0 | 0 | 0 |

By named start (the M2-G grid columns are pooled per surface as `grid`):

| Start | Gated parked runs | Clears | B defeated by Donut | A defeated by Donut |
| --- | ---: | ---: | ---: | ---: |
| M2-01 floor x1072 | 2196 | 0 | 0 | 0 |
| M2-02 shelf x1168 | 2196 | 0 | 0 | 0 |
| M2-03 shelf x1204 | 2196 | 0 | 0 | 0 |
| M2-04 floor x1248 | 2196 | 204 | 204 | 0 |
| M2-05 floor x1376 | 2196 | 360 | 360 | 0 |
| M2-06 partition x1400 | 732 | 0 | 0 | 0 |
| M2-07 floor x1424 | 2196 | 360 | 360 | 0 |
| M2-08 floor x1640 | 2196 | 360 | 360 | 0 |
| M2-09 floor x1840 | 2196 | 0 | 0 | 0 |
| M2-10 dock x1904 | 2196 | 360 | 0 | 360 |
| M2-11 dock x2032 | 2196 | 360 | 0 | 360 |
| M2-12 bumper x1872 | 2196 | 0 | 0 | 0 |
| M2-13 floor x1728 | 2196 | 360 | 360 | 0 |
| grid dock | 1764 | 1260 | 0 | 1260 |
| grid floor | 8736 | 5424 | 5424 | 0 |
| grid partition | 84 | 0 | 0 | 0 |
| grid remnant | 84 | 42 | 42 | 0 |
| grid shelf | 1008 | 0 | 0 | 0 |

`donut_level_changes` (diagnostic, finding 1): 1248 knock landings on another level in 1248 corral runs.

- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o-1.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o0.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o1.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o2.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o3.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o4.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o5.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o6.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o7.off.x1248.h0.fr.p0-0-45-75`
- level change: `corral_m2.solo_donut_parked.stationary_jump_cast.o8.off.x1248.h0.fr.p0-0-45-75`

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
| corral_m2 | solo_carl | carl_melee | off | 18 | 12 | 12 | 10 | 0 | 6 | 0 | 0 | 449-527 |
| corral_m2 | solo_carl | carl_melee | reflex | 18 | 12 | 12 | 12 | 0 | 6 | 0 | 0 | 449-527 |
| corral_m2 | solo_donut_assist | stationary_cast | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast_burst | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_assist | stationary_cast_burst | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | hold_attack_trivial | off | 1056 | 0 | 0 | 0 | 699 | 357 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | hold_attack_trivial | reflex | 1056 | 0 | 0 | 0 | 699 | 357 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | mobile_cast | off | 222 | 162 | 162 | 162 | 0 | 60 | 0 | 0 | 172-368 |
| corral_m2 | solo_donut_parked | mobile_cast | reflex | 222 | 162 | 162 | 162 | 0 | 60 | 0 | 0 | 172-368 |
| corral_m2 | solo_donut_parked | no_interaction | off | 1056 | 0 | 0 | 0 | 708 | 348 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | no_interaction | reflex | 1056 | 0 | 0 | 0 | 708 | 348 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_aimed | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_aimed | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_burst | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_cast_burst | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast | off | 14100 | 0 | 0 | 0 | 2418 | 11682 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast | reflex | 14100 | 0 | 0 | 0 | 2406 | 11694 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast_aimed | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_jump_cast_aimed | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_turret | off | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | solo_donut_parked | stationary_turret | reflex | 1056 | 0 | 0 | 0 | 0 | 1056 | 0 | 0 | - |
| corral_m2 | two_player | coop_probe | off | 222 | 66 | 66 | 39 | 39 | 117 | 0 | 0 | 213-996 |
| corral_m2 | two_player | coop_probe | reflex | 222 | 66 | 66 | 39 | 39 | 117 | 0 | 0 | 213-996 |
