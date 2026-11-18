# ENV-02 evidence: Floor 1 opening route pieces (full set)

Review renders for the [ENV-02 spec](../../02-art/env-02-opening-spec.md). Status: **full set drawn (Sonnet); independent Opus review passed with execution items, fixed in the review fix round.** Design and keys by Claude Opus, execution by Claude Sonnet, review by a separate Claude Opus agent.

These are composed PNGs and GIFs lit with the ENV-01 Pillow lighting model (`scripts/env_kit_light.py`) on the exported atlases. They are **Pillow previews, not Bevy captures and not gameplay evidence**, and nothing is wired into `floor01_scene`. The Bevy captures come when the integrator wires the route in.

Regenerate with:

```sh
python3 scripts/author_floor01_opening.py --force          # grids, .ase, manifests, contract, both layouts
bash scripts/export_assets.sh                              # atlases, validation, env-kit previews
python3 scripts/make_env02_evidence.py --out docs/evidence/env-02
```

`SHA256SUMS` lists every file here. `measurements.json` records actor readability in the S5 frame and the atlas colour counts.

| File | What it shows |
| --- | --- |
| `env02_key_sheet_1x.png`, `env02_key_sheet_3x.png` | Every ENV-02 piece by group: the shore in 4 states; the block (seated, settling, slipped, falling, landed), wall cavity and hazard grit; the timber set (the deprecated 96x176 low barricade is no longer shown); the **full-height barricade 96x280**; the alcove niche and hatch; the guild portal closed, **ajar** and open, and the sign; every frame of the dust trickle, impact dust and splinter burst loops; all 10 soffit tiles |
| `env02_route_plan_unlit.png` | The full 3584x400 route at 1x, unlit, split into two rows. Segment boundaries; the 23 marker rects and crosses colour-coded by kind (red: pursuer; orange: pursuer breakables, blocker and charge zone; magenta: fall region; cyan: safe pocket and protected exit; green: spawn, checkpoints, enemy spawns and exit; yellow: shore, block and humps; violet: clearance gauge); the pursuer path is drawn 70 px above the walking line |
| `env02_ceiling_options_unlit_2x.png` | S3 under option R (clearance 212 px) and option T (180 px), with the DOZ-01 h120 pursuer gauge (155x259 px) on the walking line and a marker at the recommended 200 px |
| `env02_lit_s1_intro_*`, `env02_lit_s2_arch_*` | S1 (Carl at the spawn, Donut behind, a goblin near the door) and S2 (the arch bay, the pursuer's entry window), each at 640x360 and 1920x1080 |
| `env02_lit_s3_chase_option_R_*`, `..._option_T_*` | S3 under each ceiling option at 640x360 and 1920x1080: Carl and Donut fleeing under the copper run, past the hatch and the timber sets, with the new soffit steps at both ends |
| `env02_lit_s4_widened_*` | S4: Carl and Donut in the alcove niche (104 px clear, too low for the pursuer), the torch, and the **full-height barricade** closing the passage up to the cornice, with a goblin in front of it |
| `env02_lit_s5_skirmish_*` | CP-03: Carl at the strike position, Donut behind him, two goblins inside the fall region and one beyond, the raking shore and the block |
| `env02_lit_s6_guild_closed_*`, `env02_lit_s6_guild_open_*` | S6: the portal closed (grille glow, light leaks, no actors) and open (warm interior and floor spill, Carl walking in, Donut following), at both sizes |
| `env02_guild_door_states_lit_2x.png`, `env02_guild_door_opening_lit_2x.gif` | The guild door closed, ajar and open at 2x, with Carl approaching, and the opening as a 3-frame GIF |
| `env02_soffit_steps_lit_3x.png` | The two steps of the lowered ceiling (left step column 61, right step column 104) under options R and T at 3x, lit |
| `env02_cp03_state_sequence_lit_2x.png` | The shore set in 6 panels at 2x: intact; hit 1 (damaged_1, block settling, dust trickle); hit 2 (damaged_2, block slipped, trickle); hit 3 (broken, block falling); landed (rubble and impact dust); and a splinter burst frame at the hit |
| `env02_cp03_sequence_lit_2x.gif` | The same sequence animated on a real timeline: splinter burst and trickle at each hit; after hit 3 the block holds for 6 ticks (100 ms), falls 140 px under 1050 px/s^2 (about 0.52 s) and lands with impact dust |

The 1920x1080 files are an exact x3 of the 640x360 view, with the light computed at output resolution (ADR-0007). Actors are frame 0 of the approved Carl 52 and Donut 40 atlases and of the GOB-01 h36 goblin candidates (mirrored to face left), placed on the walking line with 0 px sink. **Goblin positions in S1 to S4 are scale references only**: no goblin is planned there. The pursuer is not drawn (DOZ-01).

## Design fix round (Opus, after the independent review)

| File | What it shows |
| --- | --- |
| `env02_fix_shore_foot_1x_4x.png` | The re-keyed rotten shore foot in its 4 states, lit, at 1x and nearest 4x, with Carl at the strike position. It is warm wood with an eaten outline, a neck at the strike band, pale fibres and a crumbling base, and no metal-sleeve banding |
| `env02_fix_barricade_lane_lit_2x.png` | The full-height barricade redesigned to cross the walking lane (96×280, bottom on the deck's front lip). On the left it is intact, with Carl stopped at the blocker; on the right it is smashed, with boards thrown across the lane |

## Review fix round (Sonnet, after the independent Opus review)

| File | What it shows |
| --- | --- |
| `env02_cp03_states_no_actor_lit_1x.png`, `env02_cp03_states_no_actor_lit_3x.png` | The four shore states with the five block states (intact/seated, damaged_1/settling, damaged_2/slipped, broken/falling, broken/landed) in the lit frame with **no actor in front**, at 1x (gameplay scale) and 3x, so the notch, the split and the block tilt can be judged |
| `env02_key_sheet_1x.png`, `env02_key_sheet_3x.png` (regenerated) | Now include `soffit_end_l_c` and `soffit_end_r_c`, the barricade with its posts showing through the board gaps, and the redrawn angular `loose_block_landed`; the deprecated 96x176 barricade is no longer shown |
| `env02_lit_s4_widened_*`, `env02_fix_barricade_lane_lit_2x.png`, `env02_cp03_state_sequence_lit_2x.png`, `env02_cp03_sequence_lit_2x.gif` (regenerated) | The same scenes with the new barricade and landed rubble |

## Known weak spots

These are the same as in the [spec](../../02-art/env-02-opening-spec.md#known-weak-spots):

- the block's top face is deliberately dim (it faces away from the torch);
- the landed rubble and the impact dust are small at 1x; the review should check they read as the block's remains;
- the goblins have low mean luminance contrast against the lit brick;
- the pursuer's entry and wedge through the arch windows need CHASE-01 in Bevy to prove they work;
- the ajar lights are a Sonnet choice for the reviewer to confirm; the animation timings follow 60 Hz ticks (117, 83 and 67 ms per frame: 7, 5 and 4 ticks);
- the alcove darkening and Donut overlapping the right jamb can only be proved in Bevy (integrator note in the spec).
