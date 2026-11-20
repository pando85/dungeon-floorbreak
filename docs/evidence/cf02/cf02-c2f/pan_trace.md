# CF02-Q20 swap pan: velocity trace, legacy against eased

Setup (the CF02-00 baseline, `docs/evidence/cf02/audit/camera_trace.md`): solo, Donut 300 px behind Carl, Carl runs 30 ticks, the player swaps to Donut, 80 idle ticks. `dx` is the camera's left-edge step in px per tick toward Donut (negative); tick 0 is the swap tick itself. Produced by the unit test `cf02_swap_pan_trace_eases_in_and_out_and_arrives_where_the_legacy_pan_does` (`cargo test --locked --lib cf02_swap_pan_trace -- --nocapture`).

| tick from swap | legacy dx (constant cap 8) | eased dx (Q20) |
| ---: | ---: | ---: |
| 0 | -8 | -2 |
| 1 | -8 | -4 |
| 2 | -8 | -6 |
| 3 | -8 | -8 |
| 4 | -8 | -10 |
| 5 | -8 | -12 |
| 6 | -8 | -12 |
| 7 | -8 | -12 |
| 8 | -8 | -12 |
| 9 | -8 | -12 |
| 10 | -8 | -12 |
| 11 | -8 | -12 |
| 12 | -8 | -11 |
| 13 | -8 | -8 |
| 14 | -8 | -6 |
| 15 | -8 | -5 |
| 16 | -8 | -4 |
| 17 | -8 | -3 |
| 18 | -8 | -2 |
| 19 | -5 | -2 |
| 20 | 0 | -2 |
| 21 | 0 | 0 |
| 22 | 0 | 0 |
| 23 | 0 | 0 |

Legacy: a hard start at the full 8 px on the swap tick, 19 identical steps and a hard stop (157 px in 20 ticks). Eased: 2, 4, 6, 8, 10, 12 px (a ramp of 2 px per tick, no velocity step on the first tick), a 12 px cruise, then it slows with the remaining distance (`ceil(remaining / 4)`, minimum 2 px) and stops on the same column without overshoot (21 ticks for the same 157 px: this gap is short, so the cruise is brief; a 448 px swap takes about 45 ticks against the legacy 56, see `eased_pan_step_ramps_clamps_slows_and_never_overshoots`).
