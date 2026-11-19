# Silhouette-gate pack, BOSS-F01 (measurements only)

No verdict is recorded here. The gate is run by an independent top-tier agent from `silhouettes_2x.png`, `keys_lit_1x.png` and this sheet.

Images: `silhouettes_2x.png` (the six JUICE-01 base frames first, then the 16 encounter frames, flat, cyan line = contact row 135) and `keys_lit_1x.png` (the six keys at 1x on lit Floor 1 beside Donut and Carl, 640 x 360 each, facing left like the runtime).

Questions from `docs/02-art/floor-01-boss-design.md` section 9:

1. Does each key read as the same JUICE-01 creature (head, mop and ropes, ear, boils, veins, cuffs and chains, two countable fists) at 1x?
2. Can any encounter frame be mistaken for the pound tell (JUICE 8) or impact (JUICE 10)? `charge_tell_hold` keeps the cocked arm behind and the knuckles ahead; `phase_roar_hold` keeps both fists below the chest line.
3. Opaque heights within 4 px of the table, contact row 135, at least 1 px margin to the canvas edge (table below).
4. 19 palette colours, no orphan clusters, closed outline.
5. Thigh and shin proportions, a foot on every leg, no spike legs.
6. Tone: lumpy, pitiful, grotesque; `phase_roar_hold` reads as a parody flex.

| Frame | Name | Tag | Key | Opaque h | Spec h | Width | Top row |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | charge_tell_a | charge_tell |  | 93 |  | 94 | 43 |
| 1 | charge_tell_b | charge_tell |  | 86 |  | 101 | 50 |
| 2 | charge_tell_hold | charge_tell | K | 80 | 80 | 108 | 56 |
| 3 | charge_run_contact_a | charge_run | K | 90 | 90 | 115 | 46 |
| 4 | charge_run_pass_a | charge_run |  | 91 |  | 103 | 45 |
| 5 | charge_run_contact_b | charge_run | K | 92 | 92 | 100 | 44 |
| 6 | charge_run_pass_b | charge_run |  | 91 |  | 101 | 45 |
| 7 | crash_hit | crash_stun |  | 83 |  | 113 | 53 |
| 8 | crash_slump | crash_stun |  | 93 |  | 104 | 43 |
| 9 | crash_stun_hold | crash_stun | K | 72 | 72 | 83 | 64 |
| 10 | phase_roar_in | phase_roar |  | 103 |  | 77 | 33 |
| 11 | phase_roar_hold | phase_roar | K | 105 | 105 | 87 | 31 |
| 12 | phase_roar_flex | phase_roar |  | 107 |  | 95 | 29 |
| 13 | pinned_squash | pinned |  | 28 |  | 147 | 108 |
| 14 | pinned_hold | pinned | K | 56 | 56 | 135 | 80 |
| 15 | pinned_dead | pinned_dead |  | 56 |  | 135 | 80 |

Validator report: 14 pass, 0 fail, 2 skipped (`../validator/validator_summary.txt`).
