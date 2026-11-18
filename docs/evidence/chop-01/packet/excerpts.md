# Doc excerpts

## docs/02-art/floor-01-minor-enemy-concepts.md#Goblin Copper Chopper

### Goblin Copper Chopper with sidecar

**Reference:** `assets/concept/floor-01/enemies/floor01-goblin-copper-chopper-concept-approved.png`.

Owner-approved **light steam-powered goblin motorcycle/sidecar direction**, distinct from the tracked Murder Dozer: copper tubes, patched sheet metal, compact boiler, smoking exhaust, crooked headlight, exposed engine and shaky sidecar. The concept shows a goblin driver and sidecar passenger and investigates side/front/back profiles, acceleration, drifting and an exaggerated spill.

**Production guardrails:** This is a vehicle encounter, not a new humanoid species. The concept's passenger weapon, exact crew count, vehicle width, speed, collision radius, crash damage and scriptable pursuits are provisional illustrative ideas. Use the existing source character/vehicle constraints rather than deriving combat rules from the illustration. Preserve the opening Dozer's established visual identity and setpiece.


## docs/02-art/floor-01-minor-enemy-concepts.md#Shared visual direction

## Shared visual direction and source boundaries

- All three share **THE CRAWL — NO PANTS. NO MERCY.** gritty 80s/90s action sensibility and the existing catacomb environment grammar (warm torch pools, cool recessed stone, local copper/industrial props). Not medieval fantasy or a blanket neon filter.
- **Visual concepts approved.** No automatic approval of AI-in-image numeric labels, narrative claims, level mechanics or exactly depicted weapons. Separately check them against the story and encounter requirements before implementing native sprites.
- Convert from *original editable pixel grids/`.ase`* rather than cropping illustrated panels into runtime sprites. Rat readability and Chopper width need native 640×360/640×400 review against Carl 52px and Donut 40px.
- Preserve original uploaded reference images unchanged. Record provenance and SHA-256 values in the existing enemy reference manifest (see `../../../../assets/concept/floor-01/enemies/manifest.json`).
- Publication remains limited by the owner-reported art-only permission scope in the rights policy (see `../../../04-production/rights-and-release.md`); this does not clear distribution of a playable game or standalone asset pack.


## docs/02-art/doz-01-dozer-spec.md#3. Construction and parts

## 3. Construction and parts

The Dozer is a rig of parts, so a few frames give a lot of motion:

- a 9-frame belt;
- a 4-frame wheel;
- riders with their own cycles;
- a rattle that moves only the hull and the riders.

**Rig origin O** is the tread footprint midpoint on the walking line. Anchors are in px from O, with x forward and y up. A part's pivot is placed on its anchor. Facing left, mirror every anchor's x and flip each frame around its pivot. The data contract is `dozer_rig_h120.json` (see `../../../../assets/source/enemies/dozer/dozer_rig_h120.json`). `tests/test_art_dozer_doz01.py` checks that composing the parts with it reproduces the assembled machine pixel for pixel.

| z | Part | Atlas / tag | Frame | Pivot | Anchor | Frames (plan) |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | smoke puffs (world space after spawn) | `dozer_smoke` | 24 x 24 | (12, 12) | emitters below | 6 black + 5 steam |
| 1 | hull body | `dozer_hull_h120` / `body` | 175 x 160 | (87, 159) | (0, 0) | 1 (+ rattle offset) |
| 2 | tread | `dozer_tread_h120` / `roll` | 151 x 34 | (75, 33) | (0, 0) | **9** |
| 3 | engineer | `dozer_riders_h36` / `engineer_*` | 64 x 56 | (28, 55) seat | (-48, 98) | 7 (point, jab, idle 0 and 1, lever pull, lurch back and forward) |
| 3 | rider A | `dozer_riders_h36` / `rider_a_*` | 64 x 56 | (28, 55) seat | (38, 96) | 6 |
| 4 | rider B | `dozer_riders_h36` / `rider_b_*` | 64 x 56 | (28, 55) seat | (66, 96) | 6 (A's construction + B overlay) |
| 5 | hull front, cab rail | `dozer_front_cab_h120` / `front_cab` | 60 x 21 (trimmed) | (0, 20) bottom-left | (-75, 85) | 1 |
| 5 | hull front, rider tub | `dozer_front_tub_h120` / `front_tub` | 65 x 82 (trimmed) | (0, 81) bottom-left | (19, 23) | 1 |
| 6 | spiked wheel | `dozer_wheel_h120` / `spin` (`phase_0` to `phase_3`, `smear`) | 99 x 99 | (49, 49) axle | (124, 49) | 4 + 1 smear |
| 7 | near push arm, axle cap, lamp | `dozer_yoke_h120` / `static` | 64 x 44 | (54, 30) axle | (124, 49) | 1 |
| 8 | hit-spark (shared VFX) | `vfx_hit_spark_metal` (`assets/source/vfx/`) | 16 x 16 | (8, 8) | contact point | 3 |

Rig details:

- **Occluder rows.** The hull front hides every part of a rider below them: y 105 for the engineer, y 104 for riders A and B. Rider pivots are the **seat point**, not the floor. Rider-canvas rows 48 and below are always hidden. A flung pineapple may hang over the tub's back edge; that is deliberate and tested.
- **Emitters** (smoke spawn points): stack (15, 156), rear exhaust (-82, 124), kettle whistle (5, 129).
- **Lights** (optional A1 point lights, presentation only): lamp glass (98, 66) and firebox slots (-30, 49).
- **Extents:** x -84 to +172, top of the stack 156, dome 120, tub rim 104.
- **Rider spacing rule:** the brandishing rider sits at the rear seat. Its raised pineapple then swings over the boiler, not over the other rider's face.

**Atlas budget (h120).** Changes from phase 1:

- Tread: 9 x 5,134 = 46,206 px, up 30,804 px from the 3-frame plan.
- Wheel: 5 x 9,801 px with the optional smear.
- Riders: 19 x 3,584 = 68,096 px.
- Smoke: 11 x 576 px.
- Hit-spark: 3 x 256 px.

With the hull and the yoke, the split atlases total about 229k px. Trimming `front` saves about 20k.

**Damage and rattle states.** CHASE-01 does not need damage frames, because the Dozer is never defeated in the opening. The chase needs these runtime states, made from offsets and the existing frames:

- `rattle`: while moving, the hull body, hull front and riders move up 1 px for 3 ticks every 6 ticks. The tread and wheel stay on the floor. The riders follow 2 ticks late (secondary motion).
- `strain` (segment D, stuck or turning): rattle at double rate (1 px every 3 ticks), puffs from all three emitters, firebox light at full.
- A dented-front variant after a wall impact is a later decision.

