# DONUT-MOVES-ART-01 evidence: hero keys

These are key-pose previews for the [DONUT-MOVES-ART-01 spec](../../02-art/donut-moves-art-01-spec.md). They are rendered nearest-neighbour from the committed `moves_keys` grid by `python3 scripts/author_donut_moves_keys.py --sheets docs/evidence/donut-moves-art-01`.

This folder shows the two hero keys only. The in-betweens, the VFX strip, the timing GIFs and the in-scene GIFs belong to the execution step.

| File | Content |
| --- | --- |
| `keys_x6.png` | At 6x: the locomotion `idle` and `jump_rise`, the melee `hurt`, the **`evade_twist`** key, the CF02 `smug` key, and the **`blink_arrive`** key. Each tile shows its pivot |
| `keys_floor1_640x360.png`, `keys_floor1_1920x1080.png` | The lit Floor 1 composition at gameplay scale (1x and 3x), left to right: Carl idle, Donut idle, `evade_twist` lifted 10 px (the arc peak it holds), `jump_rise` lifted 10 px for comparison, and `blink_arrive` on the floor |
| `gate_silhouettes_1x_3x.png` | The silhouette-gate sheet: flat silhouettes and colour frames at 3x and 1x for `evade_twist`, `jump_rise`, `pounce_air` and `hurt` |

The lit scene uses the toolkit's Floor 1 layout and lighting pass (`scripts.pixelart.scene.Floor`). The kit atlases come from `bash scripts/export_assets.sh`.

Readability at 1x:

- `evade_twist` reads as an arched, twisting cat hop: the hump over a ducked head, the hooked tail, and the flash of a cream underside.
- It stays distinct from the long, horizontal `jump_rise` and `pounce_air`.
- `blink_arrive` reads as Donut rising, smug, out of a magenta-rimmed puddle. The arrive read in motion is judged on the execution GIFs, as spec section 11 explains.
