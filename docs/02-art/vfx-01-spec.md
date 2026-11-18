# VFX-01: bomb and explosion burst VFX (spec stub)

**Status:** design edited; four key frames (0, 3, 5, 10) corrected and frozen; silhouette-gate fixes applied and the keys re-frozen (section 17); the seven in-betweens are drawn through the generator; independent re-review pending; not wired into any Bevy binary. Sections 2, 3, 5 and 6 are the gathered facts; sections 1, 4 and 8 to 16 are the design decisions.

## 1. Scope (decided)

- **Bomb in flight (Bard):** tag `bomb_flight`, 3 frames, a lit fuse spark loop on the Bard's iron bomb; a separate projectile entity.
- **Explosion burst:** tag `burst`, 8 frames (flash, fireball, peak, cooling fireball, smoke fade). One burst serves the Bard bomb and the Rot Sticker, unscaled and without a palette variant (section 8).
- One atlas, one grid: `assets/source/vfx/explosion/vfx_01.grid.json`, 12 frames of 48 x 48, pivot (24, 24).
- Out of scope: scorch decals, floor residue, smoke columns, music-note emitters, the Bard's body frames (BARD-01), the Rot Sticker's body frames (ROT-01), any sound.

## 2. Canon citations

| Subject | Source | Fact |
| --- | --- | --- |
| Bard bomb, spawn point | `docs/02-art/bard-01-spec.md` section 3 (attack hitbox bullet) | On the last tick of frame 10 the runtime spawns the projectile at the frame-10 bomb centre **(53, 20)**, offset **(+25, -35)** from the Bard pivot; mirror x when facing left. Frame 11 has no bomb. |
| Bard follow-up | `docs/02-art/bard-01-spec.md` section 14 | "Bomb projectile and explosion VFX (proposed work order, Medium)": arc, rotation, fuse spark loop, explosion and scorch, hitboxes and timing. Explosion timing, area damage and chaining stay unapproved until that work order's gameplay spec. |
| Bard scope guardrail | `docs/02-art/bard-01-spec.md` section 1 | Separate VFX sprites (fuse-spark loops, thrown-bomb arcs, projectile and explosion sprites) are beyond the BARD-01 readability set. The static spark in frames 8 to 10 is part of the Bard sprite. |
| Bard board mood | same, section 1 | The explosion scene in the board's bottom right is mood only. Explosion timers, area damage, self-detonation and bomb chaining are not approved. |
| Rot Sticker burst frame, blast placeholder | `docs/02-art/rot-01-spec.md` section 3 | Frame 10 is the first burst frame (24 px shell, 26 px with spikes, hot core `G`); the full burst is a separate hazard-VFX work order. **Blast radius placeholder: a 40 px circle around the dome centre (12, 16), active from frame 10.** No damage or radius is approved. |
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

## 4. Not approved (unchanged by this draft)

- Damage, blast radius, area shape, fuse and flight timings, bomb speed or arc, chaining, self-detonation and knockback. The 40 px Rot radius, the damage-active ticks in section 8 and every number in the packet are placeholders.
- That a 48 x 48 burst is the final native size. It is a draft sized from the 32, 48 and 80 px candidate circles.
- The Rot Sticker blast area. Section 8 records that this art reads as a blast of about 20 px radius and asks gameplay to bring the 40 px placeholder in line with it (section 16).
- The concept boards are mood references, not accepted native art; nothing here traces them.

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

The keys are frozen (section 16) with the silhouette-gate fixes (section 17), and the in-betweens of section 11 are drawn. Next: the independent review against section 12 (lighting pass on and `--no-lighting`).

## 8. Design decisions

| Topic | Decision | Why |
| --- | --- | --- |
| Sheets and tags | One grid, two tags: `bomb_flight` (frames 0 to 2, loop) and `burst` (frames 3 to 11, once). Contact `free` for both. | One atlas keeps one palette and one frame size; the LLAMA-01 spit is the precedent (a `free` tag on the owner's grid, authored at its spawn point). |
| Frame and pivot | **48 x 48, pivot (24, 24)** (the centre). Bomb: pivot is the bomb body centre. Burst: pivot is the blast centre. Authored facing right; the bomb mirrors x with the thrower. | Burst art peaks at 40 x 38 px, inside the 32 to 48 px candidates; 4 px margin for debris, smoke may rise to y 2. |
| Bard spawn | Bomb sprite centre at the frame-10 bomb centre (53, 20) = Bard pivot offset (+25, -35), as in `bard-01-spec.md` section 3. The burst spawns at the bomb's impact point (gameplay decides it). | Canon from BARD-01; the bomb body here is the same 7 x 7 iron bomb stamp. |
| Rot Sticker burst | **Reuses the same `burst` tag unscaled, no palette variant.** Spawn at the dome centre (12, 16), i.e. Rot pivot (6, 16) offset (+6, 0). Its flame colours (`m r o g G w`) are already the burst ramp, so the same pixels read as the Rot's own glow. | Integer-scaling a drawn burst doubles pixel size and breaks the pixel grid; one atlas is enough because both bursts want the same ember-and-smoke read. |
| Rot blast size | **One burst serves both now; no `burst_large` in this work order.** The art reads as a blast of about **20 px radius** (fire about 24 px wide, smoke to about 38 px). Rule: the drawn fire and inner smoke at the peak must cover the damage area, so the player reads the danger from the art. The ROT-01 40 px radius (an 80 px circle, wider than Carl is tall) would be an invisible hit zone with this art; the recommendation to gameplay is a radius of 18 to 22 px. A `burst_large` (own 80 x 80 atlas, drawn natively, never runtime-scaled) is a follow-up only if the gameplay spec approves a radius above about 26 px. | Settled question 1 (section 16). A chaff enemy that out-blasts the Bard's bomb would also invert the Floor 1 threat ladder. |
| Bomb rotation | **Hand-drawn, no runtime rotation.** Frames 0 to 2 turn the fuse cap, cord and spark around the round body in 120 degree steps (top, lower front, lower back: atlas order 0, 1, 2 plays as a forward tumble); the body stamp and its `Q` highlight stay fixed at the upper left, because the light does not turn with the bomb. Mirror x only. | Settled question 2. Runtime rotation of a 7 x 7 sprite creates jagged mixed pixels and swings the baked highlight. |
| Draw order | The burst draws **behind the actors** (the `prop` draw slot, `scene` light group); Carl, Donut and the enemies draw over it. The bomb in flight draws with the actors. | Settled question 3: the peak drawn over Carl hides his torso (comparison panel in `key-scale.png`); behind him, his dark outline silhouettes against the fire. The `scene` group is also the only one where a later emissive registration is allowed (section 5). |
| Floor impact placement | For a bomb that lands on the floor, place the blast centre 8 px above the walking line; the fire's lower lobes then touch the floor and the smoke rises clear of it. A Rot Sticker on a wall bursts at its dome centre. | The burst is drawn nearly round so it also works against a wall. |
| Palette variant for Rot | **None.** No olive debris variant. | Settled question 5: debris chunks are 2 px and read only as dark against fire, so a hue swap is invisible at 1x and costs a second palette path; the smoke keys `s c` are already the Rot shell ramp, so the Rot burst already carries its own colours. |
| Ticks | 60 Hz. `bomb_flight` 3 x 4 = 12 ticks per loop. `burst` 2, 3, 4, 4, 5, 6, 8, 6, 4 = **42 ticks** (0.7 s; frame 10 holds 6 and the wisp frame 11 holds 4). | The 2-tick flash is an impact frame; the peak holds 4 + 4 ticks; smoke lingers 10 ticks and ends. |
| Damage-active (placeholder) | Burst-local ticks **0 to 8** (frames 3, 4 and 5, flash through peak). Nothing after the peak is damaging. The bomb in flight has no area box; its projectile hitbox is a gameplay decision. | Matches the ROT-01 placeholder "active from the first burst frame" and the rule that cooling frames are cosmetic. Not approved. |
| Playback end | `burst` plays once and the entity despawns after frame 11 (the sparse wisps); the smoke frames are not holds. | Lingering opaque smoke is capped (art-direction VFX rule). |

## 9. Palette `vfx-floor01-v0` (16 of at most 20 colours)

Shared with the Bard (`bard-floor01-v0`, same hex): `K q Q n b h Y e`. Shared with the Rot Sticker (`rot-sticker-floor01-v1`, same hex): `m r o g G w` and the smoke violets `s c` (its shell ramp `#2b2230`, `#3f3243`).

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| K | outline_soot | `#1b1124` | outline, dark debris |
| q / Q | iron dark / light | `#3a3245` `#766c86` | bomb body; debris; smoke highlight |
| n / b / h | brass dark / mid / light | `#6a4a1e` `#b8862e` `#f0cf6a` | fuse cord, fuse cap |
| e | spark halo (accent) | `#ffd23a` | fuse spark halo |
| Y | spark core (accent) | `#fff4b8` | fuse spark core, **only** in `bomb_flight` |
| s / c | smoke deep / mid | `#2b2230` `#3f3243` | smoke shade and body, motion streaks |
| m | ember dead | `#5a1218` | fireball rim, dying embers |
| r | ember red (accent) | `#a51418` | fireball edge |
| o | glow ember (accent) | `#ec3a1c` | fireball body |
| g | glow hot (accent) | `#ff8a3c` | fireball inner, fire tongues |
| G | glow core (accent) | `#ffdc8a` | hot core, frames 3 to 5 only |
| w | flash white (accent) | `#efe3d3` | flash frame only, at most 12 px |

Four free slots. The emissive keys for the A1 lighting contract would be `r o g G w` (`e` and `Y` are already the Bard's); registering them is a data change against the shared 16-colour emissive budget and is not done in this draft.

## 10. Key frames as authored

| Atlas frame | Tag / index | Key | What is drawn |
| --- | --- | --- | --- |
| 0 | `bomb_flight` 0 | key | The Bard's 7 x 7 iron bomb centred on the pivot, a brass cap and a curled `n` fuse cord, a 5 x 5 spark crown (`Y` core, `e` halo) on the tip, three soot-violet motion streaks behind and a short ember trail (`o`, `m`). |
| 3 | `burst` 0, flash | key | Compressed lumpy core (radius about 8.5 px, five lobes; `w` 9 px in one compact irregular patch, `G` 16 px), `g o r m` ramp, three uneven `o r m` fire tongues bulging out of it, seven 2 px soot/iron debris chunks and three `o` grit chunks thrown at different distances. **No radial spikes.** The hottest frame of the effect. 308 px. |
| 5 | `burst` 2, peak | key | A lumpy fire cloud about 24 px wide (`G` 21 px core, `g`, `o`, `r`, `m` rim) inside rising dark-violet smoke (five lobes above and beside the fire, none under it, ragged dithered edges, `Q` tops lit from the upper left), with two smoke rolls over the fire's crown; debris and ember chunks. **No `w`.** 772 px, bbox 38 x 39. |
| 10 | `burst` 7, smoke fade | key | Six ragged, dithered smoke puffs (`Q c s`) rising from the centre, a few dying 2 px embers and falling debris. No `w`, `G` or `g`. 468 px. |

Design edit (changes from the draft): radial spikes removed from the flash (cartoon star read); peak fireball made smaller and lumpier (947 to 772 px), smoke lobes moved above the fire and dithered at the edge, smoke drawn over the fire's crown; floating ember chunks behind the bomb replaced by a spark trail attached to the fuse; isolated non-accent pixels merged into their neighbours (`despeckle`).

All four are rasterized by `scripts/author_vfx01.py` from lumpy-disc rules with a fixed-seed integer hash; no random module, no float seeds.

## 11. In-between list for the executor (no design decisions needed)

| Slot | Currently | Target |
| --- | --- | --- |
| 1, 2 | copies of frame 0 | Same body stamp and `Q` highlight; the cap, cord and spark crown moved 120 and 240 degrees around the body by hand (atlas frame 1 lower front, frame 2 lower back, so the loop tumbles forward); the spark trail always streams behind (left); the spark crown alternates `Y`/`e` shapes (3 distinct sparks across 3 frames, clean loop seam). |
| 4 | copy of frame 3 | Fireball growing from the flash: radius 10 to 11, the tongues merging into the body, `G` core 20 to 26 px, `w` gone, first smoke lobes above. |
| 6 | copy of frame 5 | Fireball cooling: no `G` (it follows the damage window, so it no longer reads peak-hot), `g` band and more `o`, smoke lobes widen. |
| 7 | copy of frame 5 | `G` gone, one solid off-centre `g` patch high in the `o` fire (no ring, no hole, broken outline), smoke covers the top half. |
| 8 | copy of frame 5 | Fireball under 11 px radius in `o r m`, smoke dominant with a ragged edge and fire lumps showing through (no clean bowl), embers falling. |
| 11 | new | Sparse 2 px-clumped smoke wisps (about 150 px) and the last falling ember and debris; follows key 10 so the smoke thins before it despawns. |
| 9 | copy of frame 5 | A shrinking `r m` ember core inside the smoke; smoke puffs begin to dither. |

The hottest key present in a frame never rises from frame 3 to frame 11, and the smoke area grows until frame 8 and shrinks after frame 9.

## 12. Acceptance targets and validators

- **Readable at 1x beside Carl and the Bard:** the burst reads at 40 px against Carl's 52 px; the bomb (7 px body, spark crown) reads as a thrown, lit object on brick. Evidence: `docs/evidence/vfx-01/key-scale.png`.
- **No blowout:** `w` at most 12 px in frame 3 and none elsewhere; the `G` core at most 60 px (V5); whole key frames cover under 45 % of the cell (test). The glow is baked and must read with the A1 pass on and off.
- **Grounded, smoky look, not a cartoon star burst:** an irregular lumpy fire cloud with a dark rim; no radial spikes on any frame (outward motion is read from thrown debris); from the peak on, dark violet smoke is part of the silhouette; the last frame is smoke and debris with no flame.
- **Player visibility:** the burst draws behind the actors (section 8); Carl's silhouette must stay whole over every burst frame. Checked on the lit scene for the peak (`key-scale.png`, right-hand panels).
- Toolkit validators pass (`python3 -m scripts.pixelart validate assets/source/vfx/explosion/vfx_01.pixelart.json`). V05 isolated pixels: 0. V06 orphan pixels: **at most 9**, the same as the character sets (the keys measure 5, 9, 0 and 0: debris and embers are 2 px chunks and `despeckle` merges smoke noise, so effects need no looser limit). V12 unique frames: on (all 12 frames differ).
- Config rules: V1 bomb compact, V2 flash small, V3 peak 32 to 44 px wide, V4 smoke within 40 x 42, V5 white and core caps, V6 lit fuse core in the bomb frame.

## 13. Evidence

| File | Shows |
| --- | --- |
| `docs/evidence/vfx-01/key-scale.png` | Top: lit Floor 1 at 1x, Carl, the Bard (frame 10), the four keys in order bomb, flash, peak, smoke, then the peak with Carl overlapping it twice: burst behind Carl (decided) and burst over Carl (rejected, for comparison). Bottom: the four keys at 4x. |
| `docs/evidence/vfx-01/keys-x3.png` | The four frozen keys at 3x on the dark brick value, unlit. |
| `docs/evidence/vfx-01/review-pack/` | Toolkit review pack: `01_contact_sheet_x4.png` (all 12 frames), `02_lit_scene_640x360_x3.png`, `03_tell_attack_strip.png`, `validator_summary.txt`. |

Regenerate with `python3 scripts/author_vfx01.py key-scale` (needs `scripts/export_assets.sh` first) and `python3 scripts/author_vfx01.py keys-x3`. The review must also render with `--no-lighting`; not done yet.

## 14. Files

| File | Role |
| --- | --- |
| `scripts/author_vfx01.py` | authoring record: palette, rules, keys 0, 3, 5, 10, in-betweens 1, 2, 4, 6 to 9, manifest, key-scale |
| `assets/source/vfx/explosion/vfx_01.grid.json`, `vfx_01.ase`, `vfx_01.pixelart.json` | source, Aseprite file, config |
| `assets/manifests/vfx-01.json` | 60 Hz tick table and provenance |
| `tests/fixtures/vfx-01_keys.json`, `tests/test_art_vfx01.py` | frozen keys, spec tests |

## 15. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Draft spec and keys: Claude Sonnet. Design edit and key freeze: Claude Opus. Silhouette gate: independent top-tier review. Gate fixes, in-betweens and execution: Claude Sonnet. Final art review: pending. The keys are rasterized from rules in `scripts/author_vfx01.py`; no tracing of the concept, no image-model raster output, no colour sampled from the concept.

## 16. Settled questions (design edit)

1. **Rot Sticker blast size:** one burst serves both; no `burst_large` now. The art reads as about a 20 px radius, and gameplay is asked to set the Rot radius to 18 to 22 px so the drawn blast covers the damage area. A natively drawn `burst_large` follows only if gameplay approves a radius above about 26 px (section 8).
2. **Bomb in flight:** the 7 x 7 Bard bomb stays (same stamp as in the Bard's hand, readable through its outline and the bright spark). Rotation is hand-drawn in three 120 degree frames with a fixed highlight; no runtime rotation.
3. **Peak density:** reduced from 947 to 772 px, and the burst draws behind the actors, so Carl stays whole in front of it (checked on the lit scene).
4. **Orphan limit:** tightened from 16 to the standard 9.
5. **Olive debris variant for the Rot Sticker:** rejected; no variant.

## 17. Executor notes (gate fixes and in-betweens)

Gate fixes (reviewer-prescribed, so the affected keys 0, 3 and 5 are re-frozen in `tests/fixtures/vfx-01_keys.json`; key 10 is unchanged):

1. **Lobe caps.** `Canvas.cap_lobes` opens each fire cloud with a round 5 x 5 kernel (3 px flat cap, 1 px shoulders), so no lobe ends in a 1 to 2 px tip. It is in the generator, so frames 4 and 6 to 8 inherit it. Peak area stays 772 px; lobe count unchanged.
2. **Flash grit.** The hollow ring at the upper right was a stray `G` pixel at the centre of the upper-right fire tongue (the grit noise pushed `t` below the core threshold), ringed by `o`. The tongues now use core -1, so no `G` appears in a tongue; the 3 `o` grit chunks are unchanged 2 px chunks (the review located the ring at an `o` chunk, but no such ring exists there). `G` in the flash: 23 to 22 px.
3. **Rim runs.** `cap_lobes` breaks any straight 1:1 diagonal edge run of 4 px or more by filling one concave corner with `o` (one bulge per run, not per window).
4. **Bomb highlight.** `BODY7` gains a third `Q` pixel (row 2, column 2), fixed in frames 0 to 2. This differs from the Bard's hand bomb stamp by that one pixel; BARD-01 is not edited here.

In-betweens (all from generator rules, same seeds as the keys, never frozen): 1 and 2 turn the cap (`b`), cord (`n`) and spark crown to the lower back and lower front with the body and highlight fixed and three different crown shapes (`SPARK`, `SPARK1`, `SPARK2`); 4 grows the fire from the flash (radius 10.5, `G` 27 px, no `w`, tongues merged, first smoke above); 6 cools (`G` 16 px); 7 has no `G` and a `g` ring around an `o` centre; 8 is `o r m` only (radius 8) under dominant smoke; 9 is a shrinking `r m` core in dithered puffs of frame 10's layout. Smoke area (`c s Q`): 362, 443, 563, 699, 492, 456 px for frames 5 to 10, so it grows to frame 8 and shrinks after frame 9.

Ambiguities: (a) section 11 asks frame 6 for "radius 15"; read as the whole burst extent (fire plus smoke), not the fire, since a 15 px fire radius would exceed the peak and break the "hottest key never rises" rule; the fire radius is 11 (peak 11.5). (b) Frame 8's smoke rolls cover the fire's upper half, so the fire reads as a dark-red crescent under smoke; this is the intended "smoke dominant" read. (c) Frame 7's `g` ring around an `o` centre reads slightly as a donut at 4x; it is the spec's "g ring only" and is a 5-tick frame. (d) The review pack was rendered with lighting on; the `--no-lighting` render is still open.

## 18. Review fixes (blind review, execution items)

Reviewer-prescribed. Items 1 to 8 leave the four keys unchanged; items 9 and 10 (fix-check) re-freeze keys 3, 5 and 10.

1. **Bomb:** frames 1 and 2 swap to the order 0, 2, 1 (forward tumble for a right-facing throw), the cord is 2 px shorter with one `b` pixel so the spark stays attached on the warm floor.
2. **Frame 7:** one solid off-centre `g` patch, no ring; fire seed and amplitude changed for a broken outline (section 11 wording amended).
3. **Frame 8:** ragged, sparse smoke cover with fire lumps showing through (no clean bowl).
4. **Smoke despawn:** a decay step inside the 42-tick budget. Frame 10 now holds 6 ticks and a new in-between, frame 11 (sparse wisps), holds 4. The atlas has 12 frames; the manifest, `.ase`, config and tests follow. Frame 9 holds 8 ticks as before.
5. **Frame 6:** `G` removed, so only frames 3 to 5 (the damage-active placeholder window) carry the core.
6. **Evidence:** `docs/evidence/vfx-01/scene_no_lighting_1920x1080.png` (`python3 scripts/author_vfx01.py no-lighting`, A1 pass off); the review-pack lit scene now shows the peak behind Carl and the effects 8 px above the walking line with no ground shadows (`scene.shadow: false`; the toolkit scene gained a per-actor `behind` flag and an unlit option).
7. **Gameplay recommendation (replaces the 18 to 22 px range of section 8):** a damage radius of about 16 px, at most 18, because the drawn fire is about 24 x 22 px and the fire plus side smoke about 30 px wide. For 20 px or more the peak fire would need to grow to about 30 px wide.
8. **Deferred top-tier palette note, not done here:** the smoke `s c` is darker than the board's dusty mauve and reads as coal on lit floor; the option is one lighter mauve key (about `#5e4c66`, one of the four free slots) for lit tops and puff bodies of frames 8 to 11. It changes the palette, so it needs a top-tier decision.
9. **E6 flash weight (fix-check):** the flash `w` core threshold goes from 0.12 to 0.19 of the radius, so `w` is 9 px (was 3, cap 12) in one compact irregular patch, with no spikes; `G` in the flash is 16 px (was 22). Key 3 re-frozen.
10. **E7 smoke dither (fix-check):** smoke edge dither is clumped in 2 px pairs (`(x // 2 + y // 2) % 2`) in the peak, frame 10 and the in-between smoke, and the lit-top `Q` threshold is lowered (peak and in-between smoke 0.48 to 0.40, frame 10 puffs 0.70 to 0.60) so the puffs read as volumes. Keys 5 and 10 re-frozen (key 0 unchanged); the `.ase`, manifest, review pack, key-scale and keys-x3 are regenerated.
11. Open for later: D2 uneven peak lobes, D1 mauve smoke key (both top-tier).
