# VFX-01: bomb and explosion burst VFX (spec stub)

**Status:** packet ready; design pending. This file holds only the facts gathered for the designer. Sizes, frame counts, palette, timings and boxes are decided in the design step and are not decided here.

## 1. Scope

- **Bomb in flight (Bard):** the thrown bomb with a lit fuse, proposed 2 to 4 frames, a separate projectile entity.
- **Explosion burst, Bard-sized:** proposed 6 to 8 frames.
- **Rot Sticker burst:** a scale or variant of the same burst if feasible; otherwise its own sheet (the designer decides and records why).
- Out of scope unless the design step adds them: scorch decals, floor residue, smoke columns, music-note emitters, the Bard's body frames (BARD-01) and the Rot Sticker's body frames (ROT-01).

## 2. Canon citations

| Subject | Source | Fact |
| --- | --- | --- |
| Bard bomb, spawn point | `docs/02-art/bard-01-spec.md` section 3 (attack hitbox bullet) | On the last tick of frame 10 the runtime spawns the projectile at the frame-10 bomb centre **(53, 20)**, offset **(+25, -35)** from the Bard pivot; mirror x when facing left. Frame 11 has no bomb. |
| Bard follow-up | `docs/02-art/bard-01-spec.md` section 14 | "Bomb projectile and explosion VFX (proposed work order, Medium)": arc, rotation, fuse spark loop, explosion and scorch, hitboxes and timing. Explosion timing, area damage and chaining stay unapproved until that work order's gameplay spec. |
| Bard scope guardrail | `docs/02-art/bard-01-spec.md` section 1 | Separate VFX sprites (fuse-spark loops, thrown-bomb arcs, projectile and explosion sprites) are beyond the BARD-01 readability set. The static spark in frames 8 to 10 is part of the Bard sprite. |
| Bard board mood | same, section 1 | The explosion scene in the board's bottom right is mood only. Explosion timers, area damage, self-detonation and bomb chaining are not approved. |
| Rot Sticker burst frame, blast placeholder | `docs/02-art/rot-01-spec.md` section 3 (on branch `origin/art/rot-01`, not yet on `main`) | Frame 10 is the first burst frame (24 px shell, 26 px with spikes, hot core `G`); the full burst is a separate hazard-VFX work order. **Blast radius placeholder: a 40 px circle around the dome centre (12, 16), active from frame 10.** No damage or radius is approved. |
| Rot Sticker board | `docs/02-art/rot-01-spec.md` section 1 | The board's red explosion burst is a concept illustration; blast radius, damage, trigger rules and death behaviour are not approved. A dud kill leaves a skin and no burst (ROT-01 frame 14). |
| Spawn contract precedent | `docs/02-art/llama-01-spec.md` (spit) | Projectile drawn from a separate tag with the owner's pivot, authored at its spawn point; speed, range and damage are gameplay decisions, not art. Impact splash and residue VFX were out of scope there. |
| Fire and shock rule | `docs/02-art/art-direction.md` (VFX bullets) | High-contrast highlights and impact frames; cap lingering bloom and opaque smoke; lighting may use modern additive effects, but base raster layers stay intentionally pixelated. |

## 3. Concept sources and packet

Boards (1672 x 941): `assets/concept/floor-01/enemies/floor01-goblin-bomb-bard-concept-approved.png` and `assets/concept/floor-01/enemies/scatterers-rot-sticker-slime-bestiary.png`. Crops are references only; never trace them (printed scales are not native sizes). All files are in `docs/evidence/vfx-01/packet/`.

| File | Source crop (x,y,w,h) | Shows |
| --- | --- | --- |
| `bard_bomb_in_flight_crop_1.png` | Bard board 1080,100,220,140 | skull bomb leaving the hand, spark crown, two motion streaks |
| `bard_bomb_floor_spark_crop_2.png` | Bard board 1500,280,160,120 | bomb resting on the floor, lit fuse and spark spray |
| `bard_bomb_props_crop_3.png` | Bard board 1300,415,350,130 | three bomb variants (skull, banded, crate) with fuses |
| `bard_explosion_scene_crop_4.png` | Bard board 790,650,870,265 | the explosion in the arched corridor: orange-white core, ember rubble, purple smoke trail (mood only) |
| `rot_burst_crop_1.png` | Rot board 875,785,335,120 | red-orange starburst with a skull-like dark core (the explosion row) |
| `rot_glow_states_crop_2.png` | Rot board 875,540,335,125 | the dark, ember and blown-out glow states before the burst |
| `bard_palette_candidates.json`, `bard_palette_swatches.png` | Bard crops 1 to 4 | 12 quantized colours per crop |
| `rot_palette_candidates.json`, `rot_palette_swatches.png` | Rot crops 1 and 2 | 12 quantized colours per crop |
| `scale_lineup_vfx.png` | native grids | Carl 52, Donut 40, goblin 36, Bard frames 0 and 10, Rot Sticker key 0 and burst frame 10, and cyan circles at 32, 48 and 80 px diameter (candidate burst sizes; 80 px is the Rot 40 px radius placeholder) |
| `scale_lineup.png` | toolkit | the stock lineup with three empty candidate slots at 32, 48 and 80 px |
| `excerpts.md`, `spec_template.md`, `frame_pivot_template.json` | toolkit | doc excerpt, pre-filled template, frame/pivot template (the 96 x 96 frame and pivot (48, 48) are placeholders) |

The palette candidates come from the concept boards and are candidates only. The Bard sprite palette is `bard-floor01-v0` and the Rot Sticker palette `rot-sticker-floor01-v1` (glow keys `g` `#ff8a3c` and `G` `#ffdc8a`); a burst palette should reuse or extend these ramps so the VFX matches its sources.

## 4. Not approved

- Damage, blast radius, area shape, fuse and flight timings, bomb speed or arc, chaining, self-detonation and knockback. The 40 px Rot radius and every number in the packet are placeholders.
- Native sizes of the bomb and the bursts. The 32, 48 and 80 px circles in the lineup are only comparison candidates.
- Whether the Rot Sticker burst is a scale of the Bard burst or its own sheet.
- Any frame, palette or hitbox for the effect. The concept boards are mood references, not accepted native art.

## 5. A1 lighting constraints (ADR-0007, `docs/03-tech/floor01-scene.md` Lighting section)

- The A1 pass is presentation only. It evaluates lights analytically at output resolution and blooms **only the contract's emissive colours** (flame colours), with nearer opaque pixels occluding the bloom. It never feeds gameplay state.
- It does not support emitters in the `foreground` or `actors` groups, and allows at most 16 emissive colours, 64 lights and 6 slots per frame. A burst therefore cannot count on a runtime light or bloom for its glow: the sprite must read on its own with the lighting pass on and off.
- Additive-looking bursts must stay readable and must not blow out the scene: the glow is **baked into opaque or semi-transparent pixels** with a hot core and a darker outer ramp, not a large flat additive fill; the hottest value (`G`-class) appears only in the first one or two burst frames and on a small core; the burst frames after the peak cool and shrink in value; lingering glow and opaque smoke are capped (art-direction rule above).
- Bard and Rot Sticker silhouettes, Carl and Donut must stay distinguishable at 1x while the burst plays over them; the burst must not wash out the player or the floor line. Reviews must be rendered with the lighting pass on (ADR-0007 evidence rule) and also with `--no-lighting`.
- Any emissive colours the burst needs go to the lighting contract as a data change, not a redraw; that budget (16 colours) is shared with torches and flames.

## 6. Difficulty score (`docs/07-agents/pixel-art-pipeline.md`, routing by difficulty)

| Point | Result | Note |
| --- | --- | --- |
| 1. New design, not a variant of an approved asset | yes | New effect art; the Rot burst may be a variant of the Bard burst |
| 2. Native height above about 40 px | no, unless the design picks it | The Bard-sized burst candidates are 32 to 48 px; the Rot placeholder radius implies 80 px. A design above 40 px adds a point |
| 3. Complex anatomy or construction | no | |
| 4. VFX, projectiles or special states | yes | The work order is the VFX and the projectile |

**Score 2 of 4: Medium, route B + silhouette gate** (medium tier drafts size, palette, all keys and the spec; a fresh top-tier editor corrects and freezes them). It becomes 3 (Hard, B + silhouette gate) if the designer chooses a burst above about 40 px.

## 7. Next step

Design step: choose sizes and frame counts for the bomb and burst, decide the Rot Sticker relationship, write palette, pivots, spawn offsets and acceptance targets, and replace sections 1 and 4 with decisions. Execution, gate and review follow the routing doc.
