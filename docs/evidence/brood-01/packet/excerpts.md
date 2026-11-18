# Doc excerpts

## docs/02-art/floor-01-enemy-concept-roster.md#Interpretation and source guardrails

## Interpretation and source guardrails

- The early on-foot common goblins use **pineapples on wooden sticks** rather than metal maces. Common A and B are visual variants, not an approved brute class. The pot-helmet engineer is not automatically a mine-layer or combat turret.
- A generated label indicating **Scatterers explode on death** is not an approved gameplay feature.
- **Slimes split on hit** is not automatically an approved feature merely because it appears in a caption.
- Rot Stickers are **small, black, rounded wall/ceiling creatures**, not conventional large spiders; use the concept boards for mood, not every illustrative anatomical detail.
- **The Hoarder is not a goblin with a backpack.** Some ensemble imagery draws a goblin-like monster; the source identity as a transformed human requires separate narrative-aware boss production review. This does not downgrade the owner's approval of the image as visual concept direction.
- Printed scale comparisons in the generated boards are not accepted native sprite dimensions.
- No pre-Guild Donut spells, Carl shotgun, or story-premature explosive equipment because a test scene or image suggests it.

The opening art direction (see `floor-01-opening-catacomb-goblins.md`), first-floor art atlas (see `floor-01-art-atlas-v1.md`) and combat-first encounter direction (see `../01-design/combat-first-encounter-design.md`) remain authoritative for their respective domains.


## docs/02-art/scat-01-spec.md#6. Palette

## 6. Palette

Palette `scatterer-floor01-v1`: **16 keys used, at most 20.** Four slots are reserved for the Brood Guardian family (same ramps, plus e.g. a deeper `chitin_00`, a pale scar/crust and a brood-sac tone), so a shared species palette never exceeds 20. The packet candidates were used by eye: the chitin ramp comes from `#7e2824`, `#944132`, `#d5713f`, `#4a272d`; the sheen from `#db9861`, `#f0cb90`; the leg browns from `#916955`; the cold belly from `#423a45`. The navy background candidates are not used.

| Key | Name | Hex | Ramp / role | Accent (orphan-exempt) |
| --- | --- | --- | --- | --- |
| `K` | outline_char | `#1c0e12` | warm near-black outline, belly line, pronotum seam | |
| `a` | chitin_0_deep | `#3d1519` | chitin ramp 0: plate seams, lowest shell band, far legs | |
| `b` | chitin_1_shade | `#5f1d1d` | chitin ramp 1: lower shell band (the far antenna is `d` since the review fix round, `b` only where it crosses the shell) | |
| `c` | chitin_2_mid | `#84291f` | chitin ramp 2: shell body colour | |
| `d` | chitin_3_light | `#ab4329` | chitin ramp 3: upper-mid band, pronotum mid | |
| `f` | chitin_4_lit | `#d0693a` | chitin ramp 4: torch-lit upper band | |
| `h` | sheen_torch | `#ea9c5c` | specular band on the dome and pronotum top | |
| `H` | glint_cream | `#f7dc9c` | single glints, eye glint, mandibles | yes |
| `g` | leg_dark | `#4a2a28` | leg ramp 0: near tibiae | |
| `j` | leg_mid | `#8a5240` | leg ramp 1: near femurs, near antenna | |
| `s` | leg_spine | `#b98660` | leg spines (1 px ticks) | yes |
| `u` | belly_cold | `#3e2f3a` | cold ambient belly; tell belly reveal; defeat belly | |
| `v` | rim_cold | `#66545e` | cold rim light on the rear underside | |
| `e` | eye_rim | `#b81c1c` | eye lower half | yes |
| `r` | eye_glow | `#ff4a32` | eye glow (the red eye); never on the hurt or defeat-hold frames | yes |
| `w` | eye_flash | `#efe3d3` | hurt flash eye only | yes |

Lighting: A1 Floor 1 (warm torch from above, cold ambient from below). The dome bands run, from top to bottom, `h`/`f`/`d`/`c`/`c`/`b`/`a`; the cold `v` rim sits only on the rear underside. Contrast against the warm brick comes from the `K` outline, the saturated chitin (more saturated than the brick) and the `h`/`H` sheen; against the cold arch from the warm ramp itself.


## docs/02-art/scat-01-spec.md#Guardrails

### Guardrails (copied from the roster and minor-enemy docs)

- A generated label indicating **Scatterers explode on death** is not an approved gameplay feature; the bottom-row death panel (yellow splat) is deliberately left out of the crops and must not be read as an explosion mechanic. **The defeat frames have no splat, no burst, no particles, no yellow and no glow**: the roach flips onto its back and curls its legs.
- The speed-line dash, the flipped-with-"!" panel and the flattened panel are concept illustrations of states, not approved behaviors or stun mechanics. The design uses "on its back" only as the defeat pose, not as a stun state.
- Printed scale comparisons in the generated board are not accepted native sprite dimensions.
- Brood Guardian, Rot Sticker and Green Slime are out of scope here (no slime splitting, no Rot Sticker anatomy); use the board for mood only.
- Convert from original editable pixel grids / `.aseprite`, never by tracing or cropping the illustrated board.

