# AREA-HAZARDS-01B design keys and readability gate

These images are the design-step evidence for [the AREA-HAZARDS-01B spec](../../../02-art/area-hazards-01b-spec.md). They are composites, not runtime captures: nothing here is drawn by the game yet.

## How to regenerate

From the repository root, run:

`python3 scripts/hz01b_key_gate.py --room6 <room-6 still> --out docs/evidence/area-hazards-01b/keys`

## Backgrounds

| Room | Background still | How it was made |
|---|---|---|
| 4 (`f01_hazards`) | `docs/evidence/f01-danger-fx/scene/acid_full_640x360.png` | committed |
| 6 (`mastery`) | not committed, SHA-256 `f81eaa35c8f8009f17fa8ff3b96d7c904c3bade6bfcde01b689189975813772c` | a 640 x 360 capture of the room 6 open scenario at tick 700 |

**Light.** The composites approximate the scene light with a flat multiply: 0.9 on the dim floor, 1.0 near the torch. The runtime light will differ slightly.

## Images

| File | Shows |
|---|---|
| `keys_x8.png` | The two frozen keys at 8x: `acid_splash_key` (the splash crown, frame 2) and `corrosion_key` (the floor-face corrosion sample) |
| `corrosion_key_old_vs_new_x8.png` | Three face tiles under a pool edge at 8x, alternately mirrored as in the scene. Top: the first corrosion key, whose five evenly spaced pits tile into a sawtooth. Bottom: the redrawn key (HZB-AD18), with three unequal bites and offset trails |
| `gate_room4_640x360.png` | Room 4 at 1x, with: <ul><li>teal pools (HZB-AD1) and a slime-green slime standing on one of them;</li><li>Carl's hurt frame inside the splash crown;</li><li>a second splash on its own;</li><li>corrosion under two pools.</li></ul> |
| `gate_room4_right_x3.png` | The slime, pool, splash and corrosion area of room 4 at 3x |
| `gate_room6_640x360.png` | Room 6 at 1x: <ul><li>the **old** green pool under a slime, for comparison;</li><li>the teal pool under a slime, with corrosion;</li><li>a teal pool with Carl inside the splash crown.</li></ul> |
| `gate_room6_right_x3.png` | The teal pools of room 6 at 3x |
| `gate_room6_cvd_x2.png` | The room 6 composite at 2x under simulated colour vision deficiency. Rows from the top: normal, protan, deutan, tritan (Machado 2009, severity 1.0) |
| `ring_tape_old_vs_new_x3.png` | The r48 tell ring at 3x, on lit brick (top row) and dark brick (bottom row). Columns: march now, march with HZB-AD2, hot now, hot with HZB-AD2. **The preview is approximate:** it recolours band pixels outside the radius to `flame_00`, whereas the author script will change exactly the outer band row |

## Gate verdict: PASS

- **F5.** At 1x the teal pool reads as its own substance under the green slime in both rooms, while the old green pool merges into its slime. Protan and deutan vision separate the two strongly. Tritan vision separates them by saturation (ΔE00 ≥ 10.4) and by shape: the slime is an outlined mound and the pool is a flat sheet.
- **Splash.** It reads as liquid thrown up around Carl's boots. It is small next to the hero, so execution must keep the full sheet height.
- **Corrosion.** It reads as old dry stains, never as liquid.
- **F1.** The dark tape backing keeps the dashes and the hot band readable on lit brick, with no loss on dark brick.

The gate images were regenerated with the redrawn corrosion key (HZB-AD18); the verdict is unchanged. `SHA256SUMS` lists the checksum of every image.
