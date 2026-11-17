# ENV-02 evidence: Floor 1 opening route pieces (design phase 1)

Review renders for the [ENV-02 design spec](../../02-art/env-02-opening-spec.md). They are composed PNGs lit with the ENV-01 Pillow lighting model (`scripts/env_kit_light.py`) on the exported atlases. They are **not Bevy captures and not gameplay evidence**, and nothing is wired into `floor01_scene`. Status: design by Claude Opus; Sonnet execution and an independent Opus review are pending.

Regenerate with:

```sh
python3 scripts/author_floor01_opening.py --force          # grids, .ase, manifests, contract, both layouts
bash scripts/export_assets.sh                              # atlases, validation, env-kit previews
python3 scripts/make_env02_evidence.py --out docs/evidence/env-02
```

`SHA256SUMS` lists every file here. `measurements.json` records actor readability in the S5 frame and the atlas colour counts.

| File | What it shows |
| --- | --- |
| `env02_key_sheet_1x.png`, `env02_key_sheet_3x.png` | Every ENV-02 piece by group: the shore in 4 states; the block (seated, slipped, falling, landed), the wall cavity and the hazard grit; the timber set and the barricade (intact and smashed); the alcove niche and the hatch; the guild portal (closed and open) and the sign |
| `env02_route_plan_unlit.png` | The full 3584×400 route at 1x, unlit, split into two rows. Segment boundaries; marker rects and crosses colour-coded by kind (red: pursuer; orange: pursuer breakables, blocker and charge zone; magenta: fall region; cyan: safe pocket and protected exit; green: spawn, checkpoints, enemy spawns and exit; yellow: shore, block and humps; violet: clearance gauge); the pursuer path is drawn 70 px above the walking line |
| `env02_ceiling_options_unlit_2x.png` | S3 under option R (clearance 212 px) and option T (180 px), with the DOZ-01 h120 pursuer gauge (155×259 px) on the walking line and a marker at the recommended 200 px |
| `env02_lit_s5_skirmish_640x360.png`, `..._1920x1080.png` | CP-03: Carl at the strike position, Donut behind him, two goblins inside the fall region and one beyond, the raking shore and the block, the S5 arch and the portal edge |
| `env02_cp03_state_sequence_lit_2x.png` | The same set in 5 panels: intact; hit 1 (damaged_1); hit 2 (damaged_2 with the block slipped); hit 3 (broken, block falling); landed (rubble, empty socket) |
| `env02_lit_s4_widened_640x360.png`, `..._1920x1080.png` | S4: Carl and Donut on the ledge in the alcove niche (104 px clear, too low for the pursuer), the torch, and the barricade closing the passage |
| `env02_lit_s3_chase_option_R_640x360.png`, `..._option_T_640x360.png` | S3 under each ceiling option: Carl and Donut fleeing under the copper run, past the hatch and the timber sets |
| `env02_lit_s6_guild_closed_640x360.png` | The guild portal closed (grille glow, light leaks), with no actors |
| `env02_lit_s6_guild_open_640x360.png`, `..._1920x1080.png` | The portal open, with the warm interior and floor spill; Carl walking in and Donut following |

The 1920×1080 files are an exact ×3 of the 640×360 view, with the light computed at output resolution (ADR-0007). Actors are frame 0 of the approved Carl 52 and Donut 40 atlases and of the GOB-01 h36 goblin candidates (mirrored to face left), placed on the walking line with 0 px sink.

## Known weak spots

These are the same as in the [spec](../../02-art/env-02-opening-spec.md#known-weak-spots):

- the block reads flatter than the ENV-01 stonework;
- the borrowed quoin tiles at the S3 ceiling steps read as wall ends;
- the goblins have low mean luminance contrast against the lit brick;
- the pursuer's entry and wedge through the arch windows need CHASE-01 in Bevy to prove they work.
